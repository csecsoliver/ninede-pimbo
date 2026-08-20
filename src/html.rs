//! Html rendering. No stylesheet and no script anywhere in this app, so the
//! markup carries all of the meaning: headings, tables, lists and forms.

use crate::models::ItemInfo;
use axum::response::Html;

/// Escape text before it goes into a page. Everything user-supplied goes
/// through here.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The document every page is wrapped in.
pub fn page(title: &str, body: &str) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{} &mdash; ninede-pimbo</title>
</head>
<body>
{}
<hr>
<footer>
<p><small>ninede-pimbo &mdash; personal inventory management by olio.
<a href="https://q238dk0jb7.apidog.io">api docs</a></small></p>
</footer>
</body>
</html>
"#,
        esc(title),
        body
    ))
}

/// Navigation shown on the signed-in pages.
pub fn nav() -> String {
    r#"<header>
<h1>ninede-pimbo</h1>
<nav>
<ul>
<li><a href="/dash">dashboard</a></li>
<li><a href="/items/new">add an item</a></li>
<li>
<form action="/logout" method="post">
<button type="submit">log out</button>
</form>
</li>
</ul>
</nav>
</header>
<hr>
"#
    .to_string()
}

pub fn search_form(q: &str) -> String {
    format!(
        r#"<form action="/search" method="get">
<label for="q">search your items</label>
<input type="search" id="q" name="q" value="{}">
<button type="submit">search</button>
</form>
"#,
        esc(q)
    )
}

/// The item listing used by the dashboard and by the search results.
pub fn item_table(items: &[ItemInfo]) -> String {
    if items.is_empty() {
        return "<p>Nothing here.</p>\n".to_string();
    }
    let rows: String = items
        .iter()
        .map(|i| {
            format!(
                r#"<tr>
<td><a href="/items/{id}">{name}</a></td>
<td>{location}</td>
<td>{tags}</td>
<td>{seen}</td>
<td>{searching}</td>
</tr>
"#,
                id = i.id,
                name = esc(&i.name),
                location = esc(&i.location),
                tags = esc(&i.tags),
                seen = esc(&i.seen_at()),
                searching = if i.searching { "looking for it" } else { "no" },
            )
        })
        .collect();
    format!(
        r#"<table>
<caption>{} item(s)</caption>
<thead>
<tr><th scope="col">name</th><th scope="col">location</th><th scope="col">tags</th><th scope="col">last seen</th><th scope="col">searching</th></tr>
</thead>
<tbody>
{}</tbody>
</table>
"#,
        items.len(),
        rows
    )
}

/// Shown when a handler cannot do what was asked.
pub fn message_page(title: &str, message: &str, back: &str, back_label: &str) -> Html<String> {
    page(
        title,
        &format!(
            "<h1>{}</h1>\n<p>{}</p>\n<p><a href=\"{}\">{}</a></p>\n",
            esc(title),
            esc(message),
            esc(back),
            esc(back_label)
        ),
    )
}
