use actix_web::{http::header::ContentType, HttpResponse};

pub async fn change_password_form() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        r#"
        <!doctype html>
        <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8" />
                <title>Change Password</title>
            </head>
            <body>
                {error_html}
                <form action="/admin/password" method="post">
                    <label
                        >Current password
                        <input
                            type="password"
                            placeholder="Enter current password"
                            name="current_password"
                        />
                    </label>
                    <label
                        >New password
                        <input
                            type="password"
                            placeholder="Enter new password"
                            name="password"
                        />
                    </label>
                    <label
                        >Confirm new password
                        <input
                            type="password"
                            placeholder="confirm new password"
                            name="new_password_check"
                        />
                    </label>

                    <button type="submit">Change password</button>
                </form>
                <p><a href="/admin/dashboard">&lt;- Back</a></p>
            </body>
        </html>
        "#
    ))
}
