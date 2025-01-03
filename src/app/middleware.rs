use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};
use secrecy::ExposeSecret;

use super::{
    auth::{scope::AppPermission, token::AccessTokenClaims},
    error::AppError,
    extrator::AuthUser,
    ApiContext,
};

const SCHEME_PREFIX: &str = "Bearer ";

/// Login required middleware
///
/// Requires that the user must have a valid JWT Bearer token.
pub async fn login_required(
    State(ctx): State<ApiContext>,
    parts: Parts,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Get the value of the `Authorization` header, if it was sent at all.
    let auth_header = parts
        .headers
        .get(AUTHORIZATION)
        .ok_or(AppError::Unauthorized)?;

    let auth_header = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;
    if !auth_header.starts_with(SCHEME_PREFIX) {
        return Err(AppError::Unauthorized);
    }

    let token = &auth_header[SCHEME_PREFIX.len()..];

    let user = ctx
        .token_manager
        .verify::<AccessTokenClaims>(token)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let scopes =
        AppPermission::parse_permissions(&user.scope).map_err(|_| AppError::Unauthorized)?;

    tracing::Span::current().record("user_id", tracing::field::display(&user.sub));

    let auth_user = AuthUser {
        user_id: user.sub,
        session_id: user.sid,
        scopes,
    };

    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}

const API_KEY_HEADER: &str = "X-Api-Key";

/// X-Api-Key header required middleware
///
/// Requires that the user must have a valid api key.
pub async fn api_key_required(
    State(ctx): State<ApiContext>,
    parts: Parts,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Get the value of the 'X-Api-Key' header, if it was sent at all.
    let auth_api_header = parts
        .headers
        .get(API_KEY_HEADER)
        .ok_or(AppError::Unauthorized)?;

    let token = auth_api_header
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    if token != ctx.config.api_key.expose_secret() {
        return Err(AppError::Unauthorized);
    }

    tracing::Span::current().record("api_key", tracing::field::display(&token));
    Ok(next.run(req).await)
}

/// Permission middleware
///
/// Requires that the user must have the specified permissions.
/// ```rust,no_run,ignore
/// .route_layer(permission_required!(&AppPermission::UserView, &AppPermission::UserEdit))
/// ```
#[macro_export]
macro_rules! permission_required {
    ($($perm:expr),+ $(,)?) => {{
        use axum::{
            extract::Request,
            middleware::{from_fn, Next},
        };

        use $crate::{
            app::{auth::scope::AppPermission, error::AppError, extrator::AuthUser},
        };

        from_fn(|req: Request, next: Next| async move {
            let auth_user = req
                .extensions()
                .get::<AuthUser>()
                .ok_or(AppError::Unauthorized)?;

            let mut has_all_permissions = true;
            $(
                has_all_permissions = has_all_permissions &&
                    auth_user.has_permission($perm);
            )+

            if has_all_permissions {
                return Ok(next.run(req).await);
            }

            Err(AppError::Forbidden)
        })
    }};
}
