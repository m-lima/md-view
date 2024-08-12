struct Html(String);

impl Html {
    fn new() -> Self {
        Self(String::from(concat!(
            r#"
<html data-theme="dark">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>"#,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/styles/notes.css")),
            r#"
        </style>
        <script type="text/javascript">"#,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/script.js")),
            r#"
        </script>
    </head>
    <body>
        <button onClick="toggleMode()">Theme</button>
        <select
          value="notes"
          onInput="e => setStyle(e.currentTarget.value)"
        />
            <option value="default">Default</option>
            <option value="notes">Notes</option>
        </select>
        <div id="md">"#,
        )))
    }

    fn buffer(&mut self) -> &mut String {
        &mut self.0
    }

    fn done(mut self) -> String {
        self.0.push_str("</div></body></html>");
        self.0
    }
}

fn render<P: AsRef<std::path::Path>>(path: P, buffer: &mut String) -> eyre::Result<()> {
    use eyre::WrapErr;

    let path = path.as_ref();
    let file = std::fs::read(path).wrap_err_with(|| format!("Could not open `{path:?}`"))?;
    let file = String::from_utf8(file)
        .wrap_err_with(|| format!("The file `{path:?}` is not a valid UTF8"))?;

    let parser = pulldown_cmark::Parser::new_ext(&file, pulldown_cmark::Options::all());

    pulldown_cmark::html::push_html(buffer, parser);

    Ok(())
}

fn watch<P: AsRef<std::path::Path>>(
    path: P,
) -> eyre::Result<(
    std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    notify::FsEventWatcher,
)> {
    use notify::Watcher;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

    Ok((rx, watcher))
}

enum Handle {
    Empty,
    Init {
        _window: winit::window::Window,
        webview: wry::WebView,
    },
}

impl Handle {
    fn update(&self, content: &str) {
        if let Self::Init { webview, .. } = self {
            if let Err(err) = webview.evaluate_script(
                format!("updateMarkdown(`{}`)", content.replace('`', "\\`")).as_str(),
            ) {
                eprintln!("Failed to update content: {err:?}");
            }
        }
    }

    fn error(&self, error: &str) {
        if let Self::Init { webview, .. } = self {
            if let Err(err) = webview.evaluate_script(
                format!(r#"updateMarkdown('<pre style="color: red;">{error}</pre>')"#).as_str(),
            ) {
                eprintln!("Failed to update content: {err:?}");
            }
        }
    }

    fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        if let Self::Init { webview, .. } = self {
            let _ = webview.set_bounds(wry::Rect {
                position: winit::dpi::LogicalPosition::new(0, 0).into(),
                size: size.into(),
            });
        }
    }
}

struct App {
    path: String,
    buffer: String,
    change_rx: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    handle: Handle,
    _watcher: notify::FsEventWatcher,
}

impl App {
    fn new(path: String) -> eyre::Result<Self> {
        let (change_rx, watcher) = watch(&path)?;

        Ok(Self {
            path,
            buffer: String::new(),
            change_rx,
            handle: Handle::Empty,
            _watcher: watcher,
        })
    }

    fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) -> eyre::Result<()> {
        let path = self.path.clone();

        let window = event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title(&path)
                .with_resizable(true)
                .with_inner_size(winit::dpi::LogicalSize::new(1024, 768)),
        )?;

        let body = {
            let mut html = Html::new();
            render(&path, html.buffer())?;
            html.done()
        };

        let webview = wry::WebViewBuilder::new_as_child(&window)
            .with_html(body.as_str())
            .build()?;

        self.handle = Handle::Init {
            _window: window,
            webview,
        };

        Ok(())
    }

    fn changes_detected(&self) -> eyre::Result<bool> {
        let mut changed = false;
        loop {
            match self.change_rx.try_recv() {
                Ok(Ok(_)) => changed = true,
                Ok(Err(err)) => {
                    return Err(eyre::Error::from(err));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(eyre::Error::from(
                        std::sync::mpsc::TryRecvError::Disconnected,
                    ));
                }
            }
        }

        Ok(changed)
    }

    fn update(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match self.changes_detected() {
            Ok(true) => {
                self.buffer.clear();
                if let Err(err) = render(&self.path, &mut self.buffer) {
                    let err = format!("Failed to read {}: {err:?}", self.path);
                    self.handle.error(&err);
                    eprintln!("{err}");
                    return;
                }
                self.handle.update(&self.buffer);
            }
            Ok(false) => {}
            Err(err) => {
                eprintln!("{err:?}");
                event_loop.exit();
            }
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_secs(1),
        ));
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Err(err) = self.init(event_loop) {
            eprintln!("Failed to initialize window: {err:?}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let winit::event::WindowEvent::Resized(size) = event {
            self.handle.resize(size);
        }
        self.update(event_loop);
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        self.update(event_loop);
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, _event: ()) {
        self.update(event_loop);
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
        self.update(event_loop);
    }
}

fn main() -> eyre::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("Missing file parameter"))?;

    let mut app = App::new(path)?;

    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut app)?;

    Ok(())
}
