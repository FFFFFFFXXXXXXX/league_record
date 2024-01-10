/*
    Shamelessly yoinked from https://github.com/joseluisq/static-web-server
    Since it was the most responsive implementation for skipping around in a video
*/

use percent_encoding::percent_decode_str;
use std::{
    cmp,
    convert::Infallible,
    future::{ready, Future, Ready},
    io,
    net::SocketAddr,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::{
    future::{self, Either},
    ready, stream, FutureExt, Stream, StreamExt,
};
use hyper::{service::Service, Body, Request, Response, Server, StatusCode};
use tauri::{AppHandle, Manager};
use tokio::{fs::File, io::AsyncSeekExt};
use tokio_util::io::poll_read_buf;

static BUFFER_SIZE: usize = 8192;

pub fn start(app_handle: &AppHandle, folder: PathBuf, port: u16) {
    let app_handle = app_handle.clone();

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

        if let Err(e) = server.await {
            log::error!("fileserver error: {e}")
        } else {
            log::info!("fileserver gracefully shutdown")
        }
        app_handle.trigger_global("fileserver_shutdown", None);
    });
}

struct MakeFileService(PathBuf);
impl MakeFileService {
    fn new(folder: PathBuf) -> Self {
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

struct FileService(PathBuf);
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
    if !headers
        .get("host")
        .and_then(|host_header| host_header.to_str().ok())
        .and_then(|host_str| host_str.rsplit_once(':'))
        .is_some_and(|(domain, _port)| domain == "127.0.0.1")
    {
        return response_from_statuscode(StatusCode::FORBIDDEN);
    }

    // skip first '/' of uri to make a relative path
    let Ok(uri) = percent_decode_str(&req.uri().path()[1..]).decode_utf8() else {
        return response_from_statuscode(StatusCode::BAD_REQUEST);
    };
    let content_type = if uri.ends_with(".mp4") {
        "video/mp4"
    } else if uri.ends_with(".json") {
        "application/json"
    } else {
        return response_from_statuscode(StatusCode::FORBIDDEN);
    };

    folder.push(uri.as_ref());
    log::info!("fileserver file requested: {folder:?}");

    let Ok(file) = File::open(folder).await else {
        return response_from_statuscode(StatusCode::NOT_FOUND);
    };
    let file_size = match file.metadata().await {
        Ok(md) => md.len(),
        Err(_) => return response_from_statuscode(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // assume well formed range header

    let Some((start, end, len)) = headers
        .get("Range")
        .and_then(|range_header| range_header.to_str().ok())
        .and_then(|str| str.split_once('='))
        .and_then(|(_, range)| range.split_once('-'))
        .and_then(|(start, end)| Some((start.parse().unwrap_or(0u64), end.parse().unwrap_or(file_size))))
        .or(Some((0, file_size)))
        .map(|(start, end)| (start, end, end - start))
    else {
        return response_from_statuscode(StatusCode::BAD_REQUEST);
    };

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

    let buf_size = cmp::min(BUFFER_SIZE, usize::try_from(len).unwrap_or(BUFFER_SIZE));
    let response = response
        .body(Body::wrap_stream(file_stream(file, buf_size, (start, end))))
        .unwrap();
    Ok::<_, String>(response)
}

#[inline]
fn response_from_statuscode(statuscode: StatusCode) -> Result<Response<Body>, String> {
    Ok(Response::builder().status(statuscode).body(Body::empty()).unwrap())
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
