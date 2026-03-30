//! 附件 handler（D2：移动端图片收纳 / 详情查看）
//!
//! - `GET  /memos/{id}/attachments`        列出附件元数据
//! - `POST /memos/{id}/attachments?filename=a.png`  原始字节直传（Content-Type 即 MIME，
//!   不用 multipart——移动端 dio 直接发 bytes，最小依赖）
//! - `GET  /attachments/{att_id}`          下载内容（inline，图片可直接展示）

use axum::{
    body::Body,
    extract::{Path, Query, State as AxumState},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};

use super::state::{err_resp, ApiState};

#[derive(serde::Deserialize)]
pub(super) struct UploadQ {
    filename: String,
}

/// 附件元数据（对齐 db::attachments::Attachment 的移动端子集）
#[derive(serde::Serialize)]
struct AttachmentDto {
    id: i64,
    filename: String,
    file_size: i64,
    mime_type: String,
    created_at: String,
}

pub(super) async fn list_attachments(
    AxumState(state): AxumState<ApiState>,
    Path(memo_id): Path<String>,
) -> Response {
    match state.svc.list_attachments(&memo_id) {
        Ok(list) => Json(
            list.into_iter()
                .map(|a| AttachmentDto {
                    id: a.id,
                    filename: a.filename,
                    file_size: a.file_size,
                    mime_type: a.mime_type,
                    created_at: a.created_at,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn upload_attachment(
    AxumState(state): AxumState<ApiState>,
    Path(memo_id): Path<String>,
    Query(q): Query<UploadQ>,
    headers: HeaderMap,
    body: axum::extract::Request, // 取原始 body
) -> Response {
    let filename = q.filename.trim();
    if filename.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "filename 不能为空"})),
        )
            .into_response();
    }
    let mime = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // 10MB 上限（备忘附件场景足够，防误传大文件占满磁盘）
    let bytes = match axum::body::to_bytes(body.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("body 读取失败: {e}")})),
            )
                .into_response();
        }
    };

    match state.svc.upload_attachment_bytes(&memo_id, filename, &mime, &bytes) {
        Ok(id) => {
            let att = state.svc.get_attachment(id).ok().flatten();
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "id": id,
                    "memo_short_id": memo_id,
                    "filename": att.as_ref().map(|a| a.filename.clone()).unwrap_or_else(|| filename.to_string()),
                    "file_size": att.as_ref().map(|a| a.file_size).unwrap_or(bytes.len() as i64),
                    "mime_type": mime,
                })),
            )
                .into_response()
        }
        Err(e) => err_resp(e),
    }
}

pub(super) async fn delete_attachment_rest(
    AxumState(state): AxumState<ApiState>,
    Path(att_id): Path<i64>,
) -> Response {
    match state.svc.delete_attachment(att_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn download_attachment(
    AxumState(state): AxumState<ApiState>,
    Path(att_id): Path<i64>,
) -> Response {
    match state.svc.read_attachment_content(att_id) {
        Ok(Some((att, bytes))) => {
            let mut resp = Response::new(Body::from(bytes));
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                att.mime_type
                    .parse()
                    .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
            );
            let cd = format!("inline; filename=\"{}\"", att.filename.replace('"', ""));
            if let Ok(v) = cd.parse() {
                resp.headers_mut().insert(header::CONTENT_DISPOSITION, v);
            }
            resp
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "附件不存在"})),
        )
            .into_response(),
        Err(e) => err_resp(e),
    }
}
