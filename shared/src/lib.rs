use serde::{Deserialize, Serialize};

/// Represents the identifier for a node within the distributed system.
///
/// A [`NodeId`] is utilized to uniquely identify a node and can be considered
/// analogous to a vertex in the graph representing the system's structure.
///
/// # Examples
///
/// ```
/// # use renraku_shared::NodeId;
///
/// let node_id = NodeId(42);
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId(pub usize);

/// Represents a directed connection between two nodes within the distributed system.
///
/// A [`Connection`] signifies a directional link between two nodes or processes.
/// The first element within the tuple ([`NodeId`]) represents the origin node, while the second
/// element represents the destination node.
///
/// When loaded, the node with the lowest [`NodeId`] value is considered as the origin (node 0),
/// and the node with the highest [`NodeId`] value is the destination (node 1), preventing potential
/// interblocking scenarios.
///
/// # Examples
///
/// ```
/// # use renraku_shared::{Connection, NodeId};
///
/// let node_id_1 = NodeId(0);
/// let node_id_2 = NodeId(1);
/// let connection = Connection(node_id_1, node_id_2);
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection(pub NodeId, pub NodeId);
