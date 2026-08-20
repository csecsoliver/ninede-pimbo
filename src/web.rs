//! The html half of the app: plain pages and plain forms, no script, no
//! stylesheet. Browsers can only send GET and POST, so every mutation here is
//! a POST that answers with a redirect (303) to the page you should be looking
//! at afterwards.

use crate::{
    AppState, auth,
    auth::{MaybeAuth, WebAuth},
    db,
    html::{self, esc, item_table, message_page, nav, page, search_form},
    models::{CreateItemRequest, ItemInfo, SearchQuery},
};
use axum::{
    Form,
    extract::{Path, Query, State},
    http::{StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LoginForm {
    key: String,
}

#[derive(Deserialize)]
pub struct EditForm {
    name: String,
    tags: String,
    desc: String,
    loc: String,
    searching: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchingForm {
    searching: Option<String>,
}

fn checked(value: &Option<String>) -> bool {
    matches!(value.as_deref(), Some("true"))
}

fn oops() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        message_page(
            "Something broke",
            "The database did not answer. Try again in a moment.",
            "/dash",
            "back to the dashboard",
        ),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        message_page(
            "No such item",
            "That item does not exist, or it is not yours.",
            "/dash",
            "back to the dashboard",
        ),
    )
        .into_response()
}

/// `GET /` &mdash; the front door.
pub async fn index(MaybeAuth(who): MaybeAuth) -> Response {
    if who.is_some() {
        return Redirect::to("/dash").into_response();
    }
    page(
        "Personal inventory",
        &format!(
            r#"<header><h1>ninede-pimbo</h1></header>
<hr>
<main>
<p>Keep track of where your things are supposed to be, and of which ones you are
currently looking for. Stick a label with the address of an item's page on the
item itself, and whoever finds it can tell you they saw it.</p>
{}
</main>
"#,
            login_form(None)
        ),
    )
    .into_response()
}

fn login_form(error: Option<&str>) -> String {
    let problem = match error {
        Some(message) => format!("<p><strong>{}</strong></p>\n", esc(message)),
        None => String::new(),
    };
    format!(
        r#"<h2>Log in</h2>
{problem}<form action="/login" method="post">
<fieldset>
<legend>Access key</legend>
<label for="key">your access key</label>
<input type="password" id="key" name="key" required autocomplete="current-password">
<button type="submit">log in</button>
</fieldset>
</form>
"#
    )
}

/// `GET /login`
pub async fn login_page(MaybeAuth(who): MaybeAuth) -> Response {
    if who.is_some() {
        return Redirect::to("/dash").into_response();
    }
    page(
        "Log in",
        &format!(
            "<header><h1>ninede-pimbo</h1></header>\n<hr>\n<main>\n{}</main>\n",
            login_form(None)
        ),
    )
    .into_response()
}

/// `POST /login` &mdash; hand over an access key, get a cookie.
pub async fn login_submit(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    let key = form.key.trim().to_string();
    let holder = if auth::plausible_key(&key) {
        match db::user_for_key(&state.db, &key).await {
            Ok(holder) => holder,
            Err(_) => return oops(),
        }
    } else {
        None
    };

    match holder {
        Some(_) => (
            [(SET_COOKIE, auth::login_cookie(&key, state.cookie_secure))],
            Redirect::to("/dash"),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            page(
                "Log in",
                &format!(
                    "<header><h1>ninede-pimbo</h1></header>\n<hr>\n<main>\n{}</main>\n",
                    login_form(Some("That key is not valid, or it has expired."))
                ),
            ),
        )
            .into_response(),
    }
}

/// `POST /logout`
pub async fn logout(State(state): State<Arc<AppState>>) -> Response {
    (
        [(SET_COOKIE, auth::logout_cookie(state.cookie_secure))],
        Redirect::to("/"),
    )
        .into_response()
}

/// `GET /dash` &mdash; everything you own, at a glance.
pub async fn dash(State(state): State<Arc<AppState>>, who: WebAuth) -> Response {
    let items = match db::list_items(&state.db, who.user_id).await {
        Ok(items) => items,
        Err(_) => return oops(),
    };
    let account = match db::user(&state.db, who.user_id).await {
        Ok(Some(user)) => user.email.unwrap_or_else(|| format!("user #{}", user.id)),
        Ok(None) => format!("user #{}", who.user_id),
        Err(_) => return oops(),
    };
    let missing = items.iter().filter(|i| i.searching).count();

    page(
        "Dashboard",
        &format!(
            r#"{nav}<main>
<h2>Dashboard</h2>
<p>Signed in as {account}. {total} item(s) tracked, {missing} of them being looked for.</p>
{search}<hr>
{table}<p><a href="/items/new">add an item</a></p>
</main>
"#,
            nav = nav(),
            account = esc(&account),
            total = items.len(),
            missing = missing,
            search = search_form(""),
            table = item_table(&items),
        ),
    )
    .into_response()
}

/// `GET /search?q=`
pub async fn search(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Query(query): Query<SearchQuery>,
) -> Response {
    let q = query.q.unwrap_or_default();
    let results = match db::search_items(&state.db, who.user_id, q.trim()).await {
        Ok(results) => results,
        Err(_) => return oops(),
    };
    page(
        "Search",
        &format!(
            r#"{nav}<main>
<h2>Search</h2>
{search}<hr>
<h3>Results for &ldquo;{q}&rdquo;</h3>
{table}</main>
"#,
            nav = nav(),
            search = search_form(&q),
            q = esc(&q),
            table = item_table(&results),
        ),
    )
    .into_response()
}

/// `GET /items/new`
pub async fn new_item_page(_: WebAuth) -> Response {
    page(
        "Add an item",
        &format!(
            r#"{nav}<main>
<h2>Add an item</h2>
<form action="/items" method="post">
<fieldset>
<legend>The item</legend>
<p><label for="name">name</label>
<input type="text" id="name" name="name" required maxlength="200"></p>
<p><label for="loc">where it lives</label>
<input type="text" id="loc" name="loc" maxlength="200"></p>
<p><label for="tags">tags, however you like to write them</label>
<input type="text" id="tags" name="tags" maxlength="200"></p>
<p><label for="desc">description</label><br>
<textarea id="desc" name="desc" rows="4" cols="40"></textarea></p>
<p><button type="submit">add it</button></p>
</fieldset>
</form>
<p><a href="/dash">back to the dashboard</a></p>
</main>
"#,
            nav = nav()
        ),
    )
    .into_response()
}

/// `POST /items` &mdash; the form behind `/items/new`.
pub async fn create_item(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Form(form): Form<CreateItemRequest>,
) -> Response {
    let name = form.name.trim();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            message_page(
                "An item needs a name",
                "Go back and give it one.",
                "/items/new",
                "back to the form",
            ),
        )
            .into_response();
    }
    match db::create_item(
        &state.db,
        who.user_id,
        name,
        form.tags.trim(),
        form.desc.trim(),
        form.loc.trim(),
    )
    .await
    {
        Ok(id) => Redirect::to(&format!("/items/{id}")).into_response(),
        Err(_) => oops(),
    }
}

/// `GET /items/{id}` &mdash; one item, with everything you can do to it.
pub async fn item_page(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Path(id): Path<i32>,
) -> Response {
    let item = match db::item_of(&state.db, who.user_id, id).await {
        Ok(Some(item)) => item,
        Ok(None) => return not_found(),
        Err(_) => return oops(),
    };
    page(&item.name, &format!("{}{}", nav(), item_body(&item))).into_response()
}

fn item_body(item: &ItemInfo) -> String {
    let searching_action = if item.searching {
        r#"<input type="hidden" name="searching" value="false">
<button type="submit">stop looking for it</button>"#
    } else {
        r#"<input type="hidden" name="searching" value="true">
<button type="submit">i am looking for this</button>"#
    };
    let selected = |on: bool| {
        if on == item.searching {
            " selected"
        } else {
            ""
        }
    };

    format!(
        r#"<main>
<h2>{name}</h2>
<dl>
<dt>where it lives</dt><dd>{location}</dd>
<dt>tags</dt><dd>{tags}</dd>
<dt>description</dt><dd>{description}</dd>
<dt>last seen</dt><dd>{seen}</dd>
<dt>currently looking for it</dt><dd>{searching}</dd>
<dt>public page, for a label or a qr code</dt><dd><a href="/_{id}">/_{id}</a></dd>
</dl>

<h3>Quick actions</h3>
<form action="/items/{id}/seen" method="post">
<button type="submit">i just saw it &mdash; note the time</button>
</form>
<form action="/items/{id}/searching" method="post">
{searching_action}
</form>

<h3>Edit</h3>
<form action="/items/{id}" method="post">
<fieldset>
<legend>Details</legend>
<p><label for="name">name</label>
<input type="text" id="name" name="name" value="{name}" required maxlength="200"></p>
<p><label for="loc">where it lives</label>
<input type="text" id="loc" name="loc" value="{location}" maxlength="200"></p>
<p><label for="tags">tags</label>
<input type="text" id="tags" name="tags" value="{tags}" maxlength="200"></p>
<p><label for="desc">description</label><br>
<textarea id="desc" name="desc" rows="4" cols="40">{description}</textarea></p>
<p><label for="searching">looking for it</label>
<select id="searching" name="searching">
<option value="true"{on}>yes</option>
<option value="false"{off}>no</option>
</select></p>
<p><button type="submit">save</button></p>
</fieldset>
</form>

<h3>Delete</h3>
<details>
<summary>delete this item</summary>
<form action="/items/{id}/delete" method="post">
<p>This cannot be undone.</p>
<button type="submit">yes, delete it permanently</button>
</form>
</details>
<p><a href="/dash">back to the dashboard</a></p>
</main>
"#,
        id = item.id,
        name = esc(&item.name),
        location = esc(&item.location),
        tags = esc(&item.tags),
        description = esc(&item.description),
        seen = esc(&item.seen_at()),
        searching = if item.searching { "yes" } else { "no" },
        searching_action = searching_action,
        on = selected(true),
        off = selected(false),
    )
}

/// `POST /items/{id}` &mdash; save the edit form.
pub async fn update_item(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Path(id): Path<i32>,
    Form(form): Form<EditForm>,
) -> Response {
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            message_page(
                "An item needs a name",
                "Go back and give it one.",
                &format!("/items/{id}"),
                "back to the item",
            ),
        )
            .into_response();
    }
    let updated = db::update_item(
        &state.db,
        who.user_id,
        id,
        Some(name),
        Some(form.tags.trim().to_string()),
        Some(form.desc.trim().to_string()),
        Some(form.loc.trim().to_string()),
        Some(checked(&form.searching)),
    )
    .await;
    match updated {
        Ok(Some(_)) => Redirect::to(&format!("/items/{id}")).into_response(),
        Ok(None) => not_found(),
        Err(_) => oops(),
    }
}

/// `POST /items/{id}/searching`
pub async fn set_searching(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Path(id): Path<i32>,
    Form(form): Form<SearchingForm>,
) -> Response {
    let updated = db::update_item(
        &state.db,
        who.user_id,
        id,
        None,
        None,
        None,
        None,
        Some(checked(&form.searching)),
    )
    .await;
    match updated {
        Ok(Some(_)) => Redirect::to(&format!("/items/{id}")).into_response(),
        Ok(None) => not_found(),
        Err(_) => oops(),
    }
}

/// `POST /items/{id}/seen` &mdash; the owner noting they saw it.
pub async fn mark_seen(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Path(id): Path<i32>,
) -> Response {
    match db::item_of(&state.db, who.user_id, id).await {
        Ok(Some(_)) => (),
        Ok(None) => return not_found(),
        Err(_) => return oops(),
    }
    match db::mark_seen(&state.db, id).await {
        Ok(Some(_)) => Redirect::to(&format!("/items/{id}")).into_response(),
        Ok(None) => not_found(),
        Err(_) => oops(),
    }
}

/// `POST /items/{id}/delete`
pub async fn delete_item(
    State(state): State<Arc<AppState>>,
    who: WebAuth,
    Path(id): Path<i32>,
) -> Response {
    match db::delete_item(&state.db, who.user_id, id).await {
        Ok(Some(_)) => Redirect::to("/dash").into_response(),
        Ok(None) => not_found(),
        Err(_) => oops(),
    }
}

/// `GET /_{id}` &mdash; what a stranger who finds the item sees. No login.
pub async fn public_item(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> Response {
    let item = match db::item(&state.db, id).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                message_page(
                    "No such item",
                    "Nothing is registered under that address.",
                    "/",
                    "ninede-pimbo",
                ),
            )
                .into_response();
        }
        Err(_) => return oops(),
    };

    let owner = match item.user_id {
        Some(user_id) => match db::user(&state.db, user_id).await {
            Ok(Some(user)) => Some(user.email.unwrap_or_else(|| format!("user #{}", user.id))),
            Ok(None) => None,
            Err(_) => return oops(),
        },
        None => None,
    };

    let whose = match &owner {
        Some(owner) => format!("<p>This item belongs to {}.</p>", esc(owner)),
        None => {
            "<p>This item has been orphaned &mdash; we no longer know whose it is.</p>".to_string()
        }
    };
    let wanted = if item.searching {
        "<p><strong>Its owner is looking for it right now.</strong></p>"
    } else {
        "<p>Its owner is not looking for it at the moment.</p>"
    };

    page(
        &item.name,
        &format!(
            r#"<header><h1>{name}</h1></header>
<hr>
<main>
{whose}
{wanted}
<dl>
<dt>name</dt><dd>{name}</dd>
<dt>where it is supposed to be</dt><dd>{location}</dd>
<dt>last reported</dt><dd>{seen}</dd>
</dl>
<h2>Found it?</h2>
<p>Let the owner know it is still around.</p>
<form action="/_{id}/seen" method="post">
<button type="submit">i have seen this item</button>
</form>
</main>
"#,
            id = item.id,
            name = esc(&item.name),
            location = esc(&item.location),
            seen = esc(&item.seen_at()),
            whose = whose,
            wanted = wanted,
        ),
    )
    .into_response()
}

/// `POST /_{id}/seen` &mdash; a finder reporting the item. No login: this is
/// what the label on the item is for.
pub async fn public_mark_seen(State(state): State<Arc<AppState>>, Path(id): Path<i32>) -> Response {
    match db::mark_seen(&state.db, id).await {
        Ok(Some(_)) => html::page(
            "Thank you",
            &format!(
                r#"<header><h1>Thank you</h1></header>
<hr>
<main>
<p>The owner will see that this item was around just now.</p>
<p><a href="/_{id}">back to the item</a></p>
</main>
"#
            ),
        )
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            message_page(
                "No such item",
                "Nothing is registered under that address.",
                "/",
                "ninede-pimbo",
            ),
        )
            .into_response(),
        Err(_) => oops(),
    }
}
