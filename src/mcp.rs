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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub entities: Vec<EntityOutput>,
    pub relations: Vec<RelationInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityOutput {
    pub name: String,
    pub entity_type: String,
    pub observations: Vec<String>,
}
