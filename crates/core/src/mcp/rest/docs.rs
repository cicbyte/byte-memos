//! Swagger UI 页面 + OpenAPI 3.0 JSON spec（无需鉴权）

use axum::response::{IntoResponse, Json};

/// Swagger UI HTML 页面（CDN 加载，spec 指向 /api/v1/openapi.json）
pub(super) async fn swagger_ui() -> impl IntoResponse {
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>byte-link REST API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  <style>body { margin: 0; }</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin="anonymous"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: "/api/v1/openapi.json",
        dom_id: "#swagger-ui",
        deepLinking: true,
        presets: [SwaggerUIBundle.presets.apis],
        layout: "BaseLayout",
      });
    };
  </script>
</body>
</html>"##;
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

/// OpenAPI 3.0 JSON spec
pub(super) async fn openapi_spec() -> impl IntoResponse {
    let spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "byte-link REST API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "byte-link REST API — 备忘录的 CRUD + 搜索 + 分类标签 + 回收站"
        },
        "components": {
            "securitySchemes": {
                "ApiKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key"
                }
            }
        },
        "paths": {
            "/memos": {
                "get": {"summary": "备忘录列表", "parameters": [{"name": "tag", "in": "query"}, {"name": "category", "in": "query"}, {"name": "visibility", "in": "query"}, {"name": "pinned_only", "in": "query"}, {"name": "limit", "in": "query"}, {"name": "offset", "in": "query"}], "responses": {"200": {"description": "备忘录列表"}}},
                "post": {"summary": "创建备忘录", "requestBody": {"content": {"application/json": {"schema": {"type": "object", "properties": {"title": {"type": "string"}, "content": {"type": "string"}, "visibility": {"type": "string"}, "tags": {"type": "array", "items": {"type": "string"}}, "category": {"type": "string"}}, "required": ["content"]}}}}, "responses": {"201": {"description": "创建成功"}}}
            },
            "/memos/{id}": {
                "get": {"summary": "备忘录详情", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "备忘录详情"}, "404": {"description": "不存在"}}},
                "put": {"summary": "更新备忘录", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "更新成功"}}},
                "delete": {"summary": "移入回收站", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "已移入回收站"}}}
            },
            "/memos/{id}/toggle-pinned": {"post": {"summary": "切换置顶", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "切换成功"}}}},
            "/memos/{id}/visibility": {"post": {"summary": "设置可见性", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "requestBody": {"content": {"application/json": {"schema": {"type": "object", "properties": {"visibility": {"type": "string"}}, "required": ["visibility"]}}}}, "responses": {"200": {"description": "设置成功"}}}},
            "/search/memos": {"get": {"summary": "搜索备忘录", "parameters": [{"name": "q", "in": "query", "required": true, "schema": {"type": "string"}}, {"name": "limit", "in": "query", "schema": {"type": "integer"}}], "responses": {"200": {"description": "搜索结果"}}}},
            "/categories": {
                "get": {"summary": "分类列表", "responses": {"200": {"description": "分类列表"}}},
                "post": {"summary": "创建分类", "requestBody": {"content": {"application/json": {"schema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}}}}, "responses": {"201": {"description": "创建成功"}}}
            },
            "/categories/{id}": {"put": {"summary": "重命名分类", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "重命名成功"}}}, "delete": {"summary": "删除分类", "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "删除成功"}}}},
            "/stats": {"get": {"summary": "统计概览", "responses": {"200": {"description": "统计数据"}}}},
            "/tags": {"get": {"summary": "标签统计", "parameters": [{"name": "limit", "in": "query", "schema": {"type": "integer"}}], "responses": {"200": {"description": "标签列表"}}}},
            "/trash": {"get": {"summary": "回收站列表", "responses": {"200": {"description": "回收站内容"}}}, "post": {"summary": "清空回收站", "responses": {"200": {"description": "已清空"}}}},
            "/trash/{type}/{id}/restore": {"post": {"summary": "恢复条目", "parameters": [{"name": "type", "in": "path", "required": true, "schema": {"type": "string"}}, {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "恢复成功"}}}},
            "/trash/{type}/{id}": {"delete": {"summary": "彻底删除", "parameters": [{"name": "type", "in": "path", "required": true, "schema": {"type": "string"}}, {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"description": "彻底删除成功"}}}}
        }
    });
    Json(spec)
}
