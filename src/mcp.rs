use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEntitiesParams {
    pub entities: Vec<EntityInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityInput {
    pub name: String,
    pub entity_type: String,
    pub observations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationsParams {
    pub relations: Vec<RelationInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RelationInput {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AddObservationsParams {
    pub observations: Vec<ObservationInput>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ObservationInput {
    pub entity_name: String,
    pub contents: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchNodesParams {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenNodesParams {
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEntitiesParams {
    pub entity_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteObservationsParams {
    pub deletions: Vec<ObservationDeletion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDeletion {
    pub entity_name: String,
    pub observations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRelationsParams {
    pub relations: Vec<RelationInput>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityOutput {
    pub name: String,
    pub entity_type: String,
    pub observations: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub entities: Vec<EntityOutput>,
    pub relations: Vec<RelationInput>,
}

use crate::db;
use libsql::Connection;
use anyhow::Result;

pub async fn run_server(conn: Connection) -> Result<()> {
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => {
                line.clear();
                continue;
            }
        };

        let response = match handle_request(&conn, req).await {
            Ok(res) => res,
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: serde_json::Value::Null,
                result: None,
                error: Some(serde_json::json!({
                    "code": -32603,
                    "message": e.to_string()
                })),
            },
        };

        println!("{}", serde_json::to_string(&response)?);
        line.clear();
    }
    Ok(())
}

async fn handle_request(conn: &Connection, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "create_entities" => {
            let params: CreateEntitiesParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_create_entities(conn, params.entities).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "create_relations" => {
            let params: CreateRelationsParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_create_relations(conn, params.relations).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "add_observations" => {
            let params: AddObservationsParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_add_observations(conn, params.observations).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "delete_entities" => {
            let params: DeleteEntitiesParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_delete_entities(conn, params.entity_names).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "delete_observations" => {
            let params: DeleteObservationsParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_delete_observations(conn, params.deletions).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "delete_relations" => {
            let params: DeleteRelationsParams = serde_json::from_value(req.params.unwrap_or_default())?;
            db::mcp_delete_relations(conn, params.relations).await?;
            Some(serde_json::json!({ "status": "success" }))
        }
        "read_graph" => {
            let graph = db::mcp_read_graph(conn).await?;
            Some(serde_json::to_value(graph)?)
        }
        "search_nodes" => {
            let params: SearchNodesParams = serde_json::from_value(req.params.unwrap_or_default())?;
            let graph = db::mcp_search_nodes(conn, &params.query).await?;
            Some(serde_json::to_value(graph)?)
        }
        "open_nodes" => {
            let params: OpenNodesParams = serde_json::from_value(req.params.unwrap_or_default())?;
            let graph = db::mcp_open_nodes(conn, params.names).await?;
            Some(serde_json::to_value(graph)?)
        }
        _ => {
            return Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(serde_json::json!({
                    "code": -32601,
                    "message": "Method not found"
                })),
            })
        }
    };

    Ok(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id,
        result,
        error: None,
    })
}

