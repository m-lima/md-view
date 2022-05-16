use color_eyre::eyre::WrapErr;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing file paramter"))?;

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
                <title>Mr. Celo</title>
            </head>
            <body style="padding: 20px">"#,
    );
    pulldown_cmark::html::push_html(&mut output, parser);
    output.push_str(r#"</body></html>"#);

    web_view::builder()
        .title(path.as_str())
        .content(web_view::Content::Html(output.as_str()))
        .resizable(true)
        .user_data(())
        .invoke_handler(|_, _| Ok(()))
        .run()?;

    Ok(())
}
