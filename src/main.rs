use color_eyre::eyre::WrapErr;

fn render(path: &str) -> color_eyre::Result<String> {
    let file = std::fs::read(&path).wrap_err_with(|| format!("Could not open `{}`", path))?;
    let file = String::from_utf8(file)
        .wrap_err_with(|| format!("The file `{}` is not a valid UTF8", path))?;

    let parser = pulldown_cmark::Parser::new_ext(&file, pulldown_cmark::Options::all());

    let mut output = String::from(
        r#"
        <html data-theme="dark">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link rel="stylesheet" href="https://unpkg.com/@picocss/pico@latest/css/pico.min.css">
                <script type"text/javascript">
                    function looper() {
                        external.invoke('refresh');
                        setTimeout(looper, 5000);
                    }
                    looper()
                </script>
                <title>Mr. Celo</title>
            </head>
            <body style="padding: 20px">"#,
    );
    pulldown_cmark::html::push_html(&mut output, parser);
    output.push_str(r#"</body></html>"#);

    Ok(output)
}

fn metadata(path: &str) -> Option<std::time::SystemTime> {
    match std::fs::metadata(&path).and_then(|p| p.modified()) {
        Ok(modified) => Some(modified),
        Err(err) => {
            eprintln!("Failed to get modified time: {}", err);
            None
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing file paramter"))?;

    let output = render(&path)?;
    let mut last_modified = metadata(&path);

    web_view::builder()
        .title(path.as_str())
        .content(web_view::Content::Html(output.as_str()))
        .resizable(true)
        .user_data(())
        .invoke_handler(|webview, arg| {
            if arg == "refresh" {
                if let Some(modified) = metadata(&path) {
                    if let Some(last_modified_value) = last_modified {
                        if modified > last_modified_value {
                            match render(&path) {
                                Ok(html) => match webview.set_html(&html) {
                                    Ok(_) => {
                                        last_modified = Some(modified);
                                    }
                                    Err(err) => {
                                        eprintln!("Failed to update html: {}", err);
                                    }
                                },
                                Err(err) => {
                                    eprintln!("Failed to render: {}", err);
                                }
                            }
                        }
                    } else {
                        println!("Refresh outer");
                    }
                }
            }
            Ok(())
        })
        .run()?;

    Ok(())
}
