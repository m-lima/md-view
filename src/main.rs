use color_eyre::eyre::WrapErr;

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
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/style.css")),
            r#"
        </style>
        <script type="text/javascript">"#,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/script.js")),
            r#"
        </script>
    </head>
    <body>
        <button onClick="toggleMode()">Theme</button>
        <div id="md">"#,
        )))
    }

    fn buffer(&mut self) -> &mut String {
        &mut self.0
    }

    fn done(mut self) -> String {
        self.0.push_str(r#"</div></body></html>"#);
        self.0
    }
}

fn render(path: &str, buffer: &mut String) -> color_eyre::Result<()> {
    let file = std::fs::read(&path).wrap_err_with(|| format!("Could not open `{}`", path))?;
    let file = String::from_utf8(file)
        .wrap_err_with(|| format!("The file `{}` is not a valid UTF8", path))?;

    let parser = pulldown_cmark::Parser::new_ext(&file, pulldown_cmark::Options::all());

    pulldown_cmark::html::push_html(buffer, parser);

    Ok(())
}

fn watch(
    path: &str,
) -> color_eyre::Result<(
    std::sync::mpsc::Receiver<notify::RawEvent>,
    notify::FsEventWatcher,
)> {
    use notify::Watcher;

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::raw_watcher(tx)?;
    watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;

    Ok((rx, watcher))
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing file paramter"))?;

    let output = {
        let mut html = Html::new();
        render(&path, html.buffer())?;
        html.done()
    };

    let webview = web_view::builder()
        .title(path.as_str())
        .content(web_view::Content::Html(output.as_str()))
        .resizable(true)
        .user_data(())
        .invoke_handler(|_, _| Ok(()))
        .build()?;

    match watch(&path) {
        Ok((rx, watcher)) => {
            let handle = webview.handle();
            let path = path.clone();
            std::thread::spawn(move || -> color_eyre::Result<()> {
                let _watcher = watcher;
                loop {
                    rx.recv()?;
                    let mut md = String::new();
                    match render(&path, &mut md) {
                        Ok(()) => {
                            if let Err(err) = handle.dispatch(move |webview| {
                                webview
                                    .eval(&format!("updateMarkdown(`{}`)", md.replace('`', "\\`")))
                            }) {
                                eprintln!("Failed to update page: {}", err);
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to read `{}`: {}", &path, err);
                            if let Err(err) = handle.dispatch(move |webview| {
                                webview.eval(&format!(
                                    r#"updateMarkdown('<pre style="color: red;">{}</pre>')"#,
                                    err
                                ))
                            }) {
                                eprintln!("Failed to clear page: {}", err);
                            }
                        }
                    }
                }
            });
        }
        Err(err) => {
            eprintln!("Failed to register watcher: {}", err);
        }
    };

    webview.run()?;

    Ok(())
}
