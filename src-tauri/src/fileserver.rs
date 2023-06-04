/*
    Shamelessly yoinked from https://github.com/joseluisq/static-web-server
    Since it was the most responsive implementation I found when jumping around in the recordings
*/

use std::{
    cmp,
    convert::Infallible,
    future::{ready, Future, Ready},
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{
    future::{self, Either},
    ready, stream, FutureExt, Stream, StreamExt,
};
use hyper::{service::Service, Body, Request, Response, Server, StatusCode};
use tauri::{AppHandle, Manager, Runtime};
use tokio::{fs::File, io::AsyncSeekExt};
use tokio_util::io::poll_read_buf;

use crate::state::Settings;

static BUFFER_SIZE: usize = 8192;

pub fn start<R: Runtime>(app_handle: AppHandle<R>, folder: PathBuf, port: u16) {
    tauri::async_runtime::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        app_handle.once_global("shutdown_fileserver", move |_| {
            _ = tx.send(());
        });

        let server = Server::bind(&addr)
            .serve(MakeFileService::new(folder))
            .with_graceful_shutdown(async {
                _ = rx.await;
            });

        let debug = app_handle.state::<Settings>().debug_log();
        if let Err(e) = server.await {
            if debug {
                eprintln!("fileserver error: {}", e)
            }
        } else if debug {
            println!("fileserver gracefully shutdown")
        }
        app_handle.trigger_global("fileserver_shutdown", None);
    });
}

pub(crate) struct MakeFileService(PathBuf);
impl MakeFileService {
    pub fn new(folder: PathBuf) -> Self {
        Self(folder)
    }
}

impl<T: Send + 'static> Service<&T> for MakeFileService {
    type Response = FileService;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: &T) -> Self::Future {
        ready(Ok(FileService(self.0.clone())))
    }
}

pub(crate) struct FileService(PathBuf);
impl Service<Request<Body>> for FileService {
    type Response = Response<Body>;
    type Error = String;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(response(req, self.0.clone()))
    }
}

#[inline]
async fn response(req: Request<Body>, mut folder: PathBuf) -> Result<Response<Body>, String> {
    let headers = req.headers();

    // only allow connections from localhost
    let mut local_conn = false;
    if let Some(host) = headers.get("host") {
        if let Ok(host_str) = host.to_str() {
            if let Some((domain, _port)) = host_str.rsplit_once(':') {
                local_conn = domain == "127.0.0.1";
            }
        }
    }
    if !local_conn {
        return Ok(response_from_statuscode(StatusCode::FORBIDDEN));
    }

    // skip first '/' of uri to make a relative path
    let uri = &req.uri().path()[1..];
    let content_type = if uri.ends_with(".mp4") {
        "video/mp4"
    } else if uri.ends_with(".json") {
        "application/json"
    } else {
        return Ok(response_from_statuscode(StatusCode::FORBIDDEN));
    };

    folder.push(Path::new(uri));
    let Ok(file) = File::open(folder).await else {
        return Ok(response_from_statuscode(StatusCode::NOT_FOUND));
    };
    let file_size = match file.metadata().await {
        Ok(md) => md.len(),
        Err(_) => return Ok(response_from_statuscode(StatusCode::INTERNAL_SERVER_ERROR)),
    };

    // assume well formed range header
    let (start, end, len) = match headers.get("Range") {
        Some(range) => {
            let range_str = range.to_str().unwrap();
            let (_, range) = range_str.split_once('=').unwrap();
            let (start, end) = range.split_once('-').unwrap();
            let start: u64 = start.parse().unwrap_or(0);
            let end: u64 = end.parse().unwrap_or(file_size);
            (start, end, end - start)
        }
        None => (0, file_size, file_size),
    };

    // assume well formed response
    let mut response = Response::builder()
        .header("Cache-Control", "public, max-age=31536000")
        .header("Content-Type", content_type)
        .header("Content-Length", len)
        .header("Accept-Ranges", "bytes");
    if len != file_size {
        response = response
            .status(StatusCode::PARTIAL_CONTENT)
            .header("Content-Range", content_range(start, end - 1, file_size));
    };

    let buf_size = cmp::min(BUFFER_SIZE, len as usize);
    let response = response
        .body(Body::wrap_stream(file_stream(file, buf_size, (start, end))))
        .unwrap();
    Ok::<_, String>(response)
}

#[inline]
fn response_from_statuscode(statuscode: StatusCode) -> Response<Body> {
    Response::builder().status(statuscode).body(Body::empty()).unwrap()
}

#[inline]
fn content_range(start: u64, end: u64, file_size: u64) -> String {
    let mut content_range = String::from("bytes ");
    content_range.push_str(format!("{}-{}", start, end).as_str());
    content_range.push('/');
    content_range.push_str(file_size.to_string().as_str());
    content_range
}

#[inline]
fn file_stream(
    mut file: File,
    buf_size: usize,
    (start, end): (u64, u64),
) -> impl Stream<Item = Result<Bytes, io::Error>> + Send {
    let seek = async move {
        if start != 0 {
            file.seek(io::SeekFrom::Start(start)).await?;
        }
        Ok(file)
    };

    seek.into_stream()
        .map(move |result| {
            let mut buf = BytesMut::new();
            let mut len = end - start;
            let mut f = match result {
                Ok(f) => f,
                Err(f) => return Either::Left(stream::once(future::err(f))),
            };

            Either::Right(stream::poll_fn(move |cx| {
                if len == 0 {
                    return Poll::Ready(None);
                }
                if buf.capacity() - buf.len() < buf_size {
                    buf.reserve(buf_size);
                }

                let n = match ready!(poll_read_buf(Pin::new(&mut f), cx, &mut buf)) {
                    Ok(n) => n as u64,
                    Err(err) => {
                        return Poll::Ready(Some(Err(err)));
                    }
                };

                if n == 0 {
                    return Poll::Ready(None);
                }

                let mut chunk = buf.split().freeze();
                if n > len {
                    chunk = chunk.split_to(len as usize);
                    len = 0;
                } else {
                    len -= n;
                }

                Poll::Ready(Some(Ok(chunk)))
            }))
        })
        .flatten()
}
