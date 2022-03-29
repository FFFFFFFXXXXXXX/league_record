use std::time::Duration;

use windows::core::{IInspectable, Interface, Result, HSTRING};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Media;
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

use windows::Graphics::SizeInt32;
use windows::Media::Core::{
    MediaStreamSample, MediaStreamSource, MediaStreamSourceSampleRequestedEventArgs,
    MediaStreamSourceStartingEventArgs, VideoStreamDescriptor,
};
use windows::Media::MediaProperties::{
    MediaEncodingProfile, MediaEncodingSubtypes, VideoEncodingProperties, VideoEncodingQuality,
};
use windows::Media::Transcoding::{MediaTranscoder, PrepareTranscodeResult};
use windows::Storage::Streams::IRandomAccessStream;

use windows::Storage::{CreationCollisionOption, FileAccessMode, KnownFolders};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Multithread, ID3D11RenderTargetView, ID3D11Texture2D,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_FLAG,
    D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RTV_DIMENSION_TEXTURE2D,
    D3D11_TEX2D_RTV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::{
    Foundation::HINSTANCE,
    Graphics::{Direct3D, Direct3D11},
};

pub struct Recorder {
    session: GraphicsCaptureSession,
    transcoder: MediaTranscoder,
    media_stream_source: MediaStreamSource,
    media_encoder: MediaEncodingProfile,
    stream: IRandomAccessStream,
    // is_recording: bool,
    // multithread: ID3D11Multithread,
    // capture_item: GraphicsCaptureItem,
    // d3d_device: ID3D11Device,
    // d3d_context: ID3D11DeviceContext,
    // direct3d_device: IDirect3DDevice,
    // compose_texture: ID3D11Texture2D,
    // render_target_view: ID3D11RenderTargetView,
    // frame_pool: Direct3D11CaptureFramePool,
    // media_stream_source: MediaStreamSource,
}

impl Recorder {
    pub fn new(window_handle: HWND) -> Result<Recorder> {
        // capture item
        let capture_item = create_capture_item_for_window(window_handle)?;
        let capture_item_size = get_capture_item_size(capture_item.Size()?);

        // d3d and direct3d
        let d3d_device = create_d3d_device();
        let d3d_context = get_d3d_context(&d3d_device);
        let direct3d_device = create_direct3d_device(&d3d_device);

        // media encoder
        let media_encoder =
            setup_media_encoder(&capture_item_size).expect("error creating media encoding profile");

        // transcoder
        let transcoder = create_media_transcoder();
        transcoder.SetHardwareAccelerationEnabled(true)?;

        // multithread
        let multithread: ID3D11Multithread = d3d_context
            .cast()
            .expect("error getting d3d11multithread from d3dcontext");
        unsafe {
            multithread.SetMultithreadProtected(true);
        }

        // file stream
        let stream = create_stream().expect("error creating filestream");

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            &capture_item_size,
        )
        .expect("error creating framepool");

        let c_session = frame_pool
            .CreateCaptureSession(&capture_item)
            .expect("error creating capture session");
        match c_session.SetIsBorderRequired(false) {
            Ok(_) => println!("yellow border removed?"),
            Err(e) => println!("error removing yellow border: {}", e),
        }

        // media stream source
        let media_stream_source = get_media_stream_source(&capture_item_size)
            .expect("error creating media stream source");

        let fp = frame_pool.clone();
        media_stream_source.Starting(
            TypedEventHandler::<_, MediaStreamSourceStartingEventArgs>::new(move |_, args| {
                println!("starting");
                args.as_ref()
                    .unwrap()
                    .Request()?
                    .SetActualStartPosition(fp.TryGetNextFrame()?.SystemRelativeTime()?)?;
                Ok(())
            }),
        )?;

        let fp = frame_pool.clone();
        media_stream_source.SampleRequested(TypedEventHandler::<
            MediaStreamSource,
            MediaStreamSourceSampleRequestedEventArgs,
        >::new(move |_, args| {
            dbg!("sample");

            match fp.TryGetNextFrame() {
                Ok(frame) => {
                    let sample = MediaStreamSample::CreateFromDirect3D11Surface(
                        frame.Surface()?,
                        frame.SystemRelativeTime()?,
                    )?;
                    args.as_ref().unwrap().Request()?.SetSample(sample)?;
                }
                Err(_) => args.as_ref().unwrap().Request()?.SetSample(None)?,
            }

            /*
            let acc: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
            let src_texture = unsafe { acc.GetInterface::<ID3D11Texture2D>()? };
            unsafe {
                d3d_context.CopyResource(
                    Some(texture.lock().unwrap().cast()?),
                    Some(src_texture.cast()?),
                );
            }
            */

            Ok(())
        }))?;

        /*

            SPACER

        */

        frame_pool.FrameArrived(
            TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(|a, _| {
                let frame = a.as_ref().unwrap().TryGetNextFrame()?;
                frame.Close()?;
                Ok(())
            }),
        )?;

        // texture and rendertargetview
        let compose_texture = create_compose_texture(&d3d_device, capture_item_size)
            .expect("error creating compose texture");
        let render_target_view = create_render_target_view(&d3d_device, &compose_texture)
            .expect("error creating render target view");
        unsafe {
            let rgba: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
            d3d_context.ClearRenderTargetView(&render_target_view, rgba.as_ptr());
        }

        let rec = Recorder {
            session: c_session,
            transcoder: transcoder,
            media_stream_source: media_stream_source,
            media_encoder: media_encoder,
            stream: stream,
            // is_recording: false,
            // multithread: multithread,
            // capture_item: capture_item,
            // d3d_device: d3d_device,
            // d3d_context: d3d_context,
            // direct3d_device: direct3d_device,
            // compose_texture: compose_texture,
            // render_target_view: render_target_view,
            // frame_pool: frame_pool,
            // media_stream_source: media_stream_source,
        };

        Ok(rec)
    }

    pub fn start_recording(&self) {
        println!("started");
        self.session.StartCapture().unwrap();
        println!("started");
        let transcoder = self
            .transcoder
            .PrepareMediaStreamSourceTranscodeAsync(
                self.media_stream_source.clone(),
                self.stream.clone(),
                self.media_encoder.clone(),
            )
            .unwrap()
            .get()
            .unwrap();
        transcoder.TranscodeAsync().unwrap();
        println!("started");
    }

    pub fn stop_recording(&self) {
        println!("stopped");
        self.session.Close().unwrap();
        println!("stopped");
    }
}

fn create_render_target_view(
    d3d_device: &ID3D11Device,
    compose_texture: &ID3D11Texture2D,
) -> Result<ID3D11RenderTargetView> {
    // might need to change to texture2darray here
    let mut desc = D3D11_RENDER_TARGET_VIEW_DESC::default();
    desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    desc.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2D;
    desc.Anonymous = D3D11_RENDER_TARGET_VIEW_DESC_0 {
        Texture2D: D3D11_TEX2D_RTV::default(),
    };
    let desc = &desc as *const _;

    unsafe {
        let render_target_view = d3d_device.CreateRenderTargetView(compose_texture, desc)?;
        Ok(render_target_view)
    }
}

fn create_compose_texture(d3d_device: &ID3D11Device, size: SizeInt32) -> Result<ID3D11Texture2D> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    desc.Width = size.Width.try_into().unwrap();
    desc.Height = size.Height.try_into().unwrap();
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    desc.SampleDesc = DXGI_SAMPLE_DESC {
        Count: 1,
        Quality: 0,
    };
    desc.Usage = D3D11_USAGE_DEFAULT;
    desc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG::default();

    unsafe {
        let texture = d3d_device.CreateTexture2D(&desc, std::ptr::null())?;
        Ok(texture)
    }
}

fn create_capture_item_for_window(
    window_handle: HWND,
) -> windows::core::Result<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForWindow(window_handle) }
}

fn get_capture_item_size(size: SizeInt32) -> SizeInt32 {
    SizeInt32 {
        Width: if size.Width % 2 == 0 {
            size.Width
        } else {
            size.Width + 1
        },
        Height: if size.Height % 2 == 0 {
            size.Height
        } else {
            size.Height + 1
        },
    }
}

fn create_d3d_device() -> ID3D11Device {
    let mut device = None;
    let _result = unsafe {
        Direct3D11::D3D11CreateDevice(
            None,
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            None,
            Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            &[],
            Direct3D11::D3D11_SDK_VERSION,
            &mut device,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    device.expect("failed creating d3d11device")
}

fn get_d3d_context(d3d_device: &ID3D11Device) -> ID3D11DeviceContext {
    unsafe {
        let mut d3d_context = None;
        d3d_device.GetImmediateContext(&mut d3d_context);
        d3d_context.unwrap()
    }
}

fn create_direct3d_device(d3d_device: &ID3D11Device) -> IDirect3DDevice {
    let dxgi_device: IDXGIDevice = d3d_device
        .cast()
        .expect("failed casting d3d11device to idxgidevice");
    let inspectable = unsafe {
        CreateDirect3D11DeviceFromDXGIDevice(Some(dxgi_device))
            .expect("error creating direct3ddevice")
    };
    inspectable
        .cast()
        .expect("failed casting inspectable to direct3ddevice")
}

fn setup_media_encoder(capture_item_size: &SizeInt32) -> Result<MediaEncodingProfile> {
    let framerate = 30;
    let bitrate = MediaEncodingProfile::CreateMp4(VideoEncodingQuality::HD1080p)?
        .Video()?
        .Bitrate()?;
    let encoding_profile = MediaEncodingProfile::new()?;
    encoding_profile
        .Container()?
        .SetSubtype(HSTRING::from("MPEG4"))?;
    encoding_profile
        .Video()?
        .SetSubtype(HSTRING::from("H264"))?;
    encoding_profile
        .Video()?
        .SetWidth(capture_item_size.Width.try_into().unwrap())?;
    encoding_profile
        .Video()?
        .SetHeight(capture_item_size.Height.try_into().unwrap())?;
    encoding_profile.Video()?.SetBitrate(bitrate)?;
    encoding_profile
        .Video()?
        .FrameRate()?
        .SetNumerator(framerate)?;
    encoding_profile.Video()?.FrameRate()?.SetDenominator(1)?;
    encoding_profile
        .Video()?
        .PixelAspectRatio()?
        .SetNumerator(1)?;
    encoding_profile
        .Video()?
        .PixelAspectRatio()?
        .SetDenominator(1)?;
    Ok(encoding_profile)
}

fn get_media_stream_source(capture_item_size: &SizeInt32) -> Result<MediaStreamSource> {
    let video_properties = VideoEncodingProperties::CreateUncompressed(
        MediaEncodingSubtypes::Bgra8()?,
        capture_item_size.Width.try_into().unwrap(),
        capture_item_size.Height.try_into().unwrap(),
    )
    .expect("error creating video encoding properties");
    let video_descriptor = VideoStreamDescriptor::Create(video_properties)?;
    let media_stream_source = MediaStreamSource::CreateFromDescriptor(video_descriptor)?;
    media_stream_source.SetBufferTime(TimeSpan::from(Duration::ZERO))?;
    Ok(media_stream_source)
}

fn create_media_transcoder() -> MediaTranscoder {
    let transcoder = MediaTranscoder::new().unwrap();
    transcoder.SetHardwareAccelerationEnabled(true).unwrap();
    return transcoder;
}

fn create_stream() -> Result<IRandomAccessStream> {
    let folder = KnownFolders::VideosLibrary()?;
    let filename = chrono::offset::Local::now().format("%Y-%m-%d_%H-%M-%S.mp4");
    let filename = format!("{}", filename);

    let file = folder
        .CreateFileAsync(
            HSTRING::from(filename),
            CreationCollisionOption::GenerateUniqueName,
        )?
        .get()?;

    let stream = file.OpenAsync(FileAccessMode::ReadWrite)?.get()?;

    Ok(stream)
}
