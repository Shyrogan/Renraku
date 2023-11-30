use std::{collections::HashSet, fs::File, io::Read, str::FromStr};

use color_eyre::{Report, Result};
use renraku_shared::{Connection, NodeId};
use thiserror::Error;

/// Represents a graph structure within the `renraku_coordinator` crate.
///
/// The [`Graph`] struct encapsulates a graph composed of vertices ([`NodeId`]) and edges ([`Connection`]).
///
/// # Examples
///
/// ```
/// # use renraku_coordinator::Graph;
/// # use renraku_shared::{NodeId, Connection};
/// use std::collections::HashSet;
///
/// let vertices: HashSet<NodeId> = HashSet::new();
/// let edges: HashSet<Connection> = HashSet::new();
/// let graph = Graph { vertices, edges };
/// ```
#[derive(Debug, Clone)]
pub struct Graph {
    pub vertices: HashSet<NodeId>,
    pub edges: HashSet<Connection>,
}

/// Enumerates each type of line in a graph file
enum Line {
    Comment(String),
    Manifest(usize, usize),
    Edge(usize, usize),
}

#[derive(Error, Debug)]
pub enum LineParsingError {
    #[error("Attempting to parse an empty line")]
    EmptyLine,
    #[error("Unknown marker '{1}' for line \"{0}\"")]
    UnknownMarker(String, char),
    #[error("Unexpected arguments length (actual: {1}, expected: {2}) in \"{0}\"")]
    UnexpectedArguments(String, usize, usize),
}

impl FromStr for Line {
    type Err = LineParsingError;

    fn from_str(s: &str) -> Result<Self, LineParsingError> {
        let first_char = s.chars().next().ok_or(LineParsingError::EmptyLine)?;

        match first_char {
            // Comments are skipped
            'c' => Ok(Self::Comment(s.to_string())),
            'p' => {
                // p edge X X
                let hints: Vec<usize> = s
                    .split_whitespace()
                    .skip(2)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if hints.len() == 2 {
                    Ok(Self::Manifest(
                        hints.get(0).unwrap().clone(),
                        hints.get(1).unwrap().clone(),
                    ))
                } else {
                    Err(LineParsingError::UnexpectedArguments(
                        s.into(),
                        2,
                        hints.len(),
                    ))
                }
            }
            'e' => {
                // e X X
                let hints: Vec<usize> = s
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();

                if hints.len() == 2 {
                    Ok(Self::Edge(
                        hints.get(0).unwrap().clone(),
                        hints.get(1).unwrap().clone(),
                    ))
                } else {
                    Err(LineParsingError::UnexpectedArguments(
                        s.into(),
                        2,
                        hints.len(),
                    ))
                }
            }
            _ => Err(LineParsingError::UnknownMarker(s.into(), first_char)),
        }
    }
}

#[derive(Error, Debug)]
pub enum GraphParsingError {
    #[error(
        "The file has been read without throwing an error yet the graph is uninitialized (is it a graph) ?"
    )]
    InvalidGraph,
    #[error(
        "An edge has been read when the graph is not yet initialized (there must be a line that starts with p before edges)"
    )]
    Uninitialized,
    #[error(transparent)]
    LineParsing(#[from] LineParsingError),
}

impl FromStr for Graph {
    type Err = GraphParsingError;

    fn from_str(s: &str) -> Result<Self, GraphParsingError> {
        let mut vertices: Option<HashSet<NodeId>> = None;
        let mut edges: Option<HashSet<Connection>> = None;

        let lines: Vec<Result<Line, LineParsingError>> = s
            .split("\n")
            .filter(|s| !s.is_empty())
            .map(|s| Line::from_str(s))
            .collect();

        for line in lines {
            match line? {
                Line::Comment(_) => {}
                Line::Manifest(v, e) => {
                    vertices = Some(HashSet::with_capacity(v));
                    edges = Some(HashSet::with_capacity(e));
                }
                Line::Edge(v1, v2) => {
                    vertices
                        .as_mut()
                        .ok_or(GraphParsingError::Uninitialized)?
                        .insert(NodeId(v1));
                    vertices
                        .as_mut()
                        .ok_or(GraphParsingError::Uninitialized)?
                        .insert(NodeId(v2));
                    edges
                        .as_mut()
                        .ok_or(GraphParsingError::Uninitialized)?
                        .insert(Connection(NodeId(v1.min(v2)), NodeId(v1.max(v2))));
                }
            }
        }

        Ok(Self {
            vertices: vertices.ok_or(GraphParsingError::InvalidGraph)?,
            edges: edges.ok_or(GraphParsingError::InvalidGraph)?,
        })
    }
}

impl TryFrom<File> for Graph {
    type Error = Report;

    fn try_from(mut value: File) -> Result<Self> {
        let mut buffer = String::new();
        value.read_to_string(&mut buffer)?;
        Ok(buffer.parse()?)
    }
}
