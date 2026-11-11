#![allow(dead_code)]
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use umbriel_client::Observation;
use zk_desktop::*;
use zk_umbriel::Reducer;

pub fn window(id: &str, workspace: &str, row: i32) -> Value {
    json!({"id":id,"workspace":workspace,"scratchpad":"","active":row == 0,
        "app_id":"app","title":"title","xdg_tag":"","content_type":"none",
        "floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,
        "x":20,"y":40 + row * 320,"w":800,"h":300,"output":"OUT","parent":"",
        "column":0,"row":row,"tabbed":false,"tab_index":-1,"tab_hidden":false,
        "title_bar":true,"fullscreen":false,"maximized":false,"maximized_to_edges":false,
        "pinned":false,"visible":true,"minimizable":true,"minimized":false,"region":null})
}
pub fn workspace(id: &str, output: &str) -> Value {
    json!({"id":id,"name":id,"named":true,"index":1,"output":output,
        "active":true,"focused":true,"occupied":true,"layout":"scrolling",
        "window_count":2,"focused_window":"upper","urgent":false,"layout_override":false})
}
pub fn output(name: &str) -> Value {
    json!({"name":name,"enabled":true,"powered":true,"focused":true,
        "active_workspace":"ws","position":{"x":0,"y":0},
        "logical_size":{"width":1920,"height":1080},
        "usable_area":{"x":0,"y":30,"width":1920,"height":1050},
        "scale":1.0,"transform":"normal"})
}
pub fn frame(epoch: u64, position: u64, family: &str, data: Value) -> Observation {
    Observation::Frame {
        epoch,
        position,
        bytes: Arc::from(serde_json::to_vec(&json!({"event":family,"data":data})).unwrap()),
    }
}
pub struct Harness {
    pub reducer: Reducer,
    pub epoch: u64,
    position: u64,
}
impl Harness {
    pub fn new() -> Self {
        let mut h = Self {
            reducer: Reducer::new(SourceId::new(Backend::Umbriel, "stock")),
            epoch: 1,
            position: 0,
        };
        h.reducer
            .apply(Duration::ZERO, Observation::Epoch { epoch: 1 })
            .unwrap();
        h
    }
    pub fn send(&mut self, family: &str, data: Value) -> DesktopUpdate {
        self.position += 1;
        self.reducer
            .apply(
                Duration::ZERO,
                frame(self.epoch, self.position, family, data),
            )
            .unwrap()
            .unwrap()
    }
    pub fn ready() -> Self {
        let mut h = Self::new();
        h.send(
            "windows",
            json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
        );
        h.send("workspaces", json!([workspace("ws", "OUT")]));
        h.send("outputs", json!([output("OUT")]));
        h
    }
    pub fn request(&self, fraction: f64) -> RelativeEdgeRequest {
        let c = self.reducer.capture();
        request(&c, fraction)
    }
}
pub fn request(c: &DesktopCapture, fraction: f64) -> RelativeEdgeRequest {
    let g = c.groups.values().next().unwrap();
    RelativeEdgeRequest {
        group: g.key.clone(),
        target: g.members[0].clone(),
        following: g.members[1].clone(),
        basis: c.version,
        axis: g.secondary_axis.unwrap_or(Axis::Vertical),
        fraction,
    }
}
