use std::collections::{BTreeMap, BTreeSet};
use std::default::Default;
use std::error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidFormat,
    RouteConflict,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidFormat => write!(f, "invalid format"),
            Error::RouteConflict => write!(f, "route conflict"),
        }
    }
}

enum NodeKind {
    Static,
    Param,
    CatchAll,
}

impl Default for NodeKind {
    fn default() -> NodeKind {
        NodeKind::Static
    }
}

struct Node<T> {
    kind: NodeKind,
    name: String,
    data: Option<T>,
    params: Vec<String>,
    normal_children: Vec<Node<T>>,
    param_child: Box<Option<Node<T>>>,
    catch_all_child: Box<Option<Node<T>>>,
}

impl<T> Default for Node<T> {
    fn default() -> Node<T> {
        Node::<T> {
            kind: NodeKind::default(),
            name: String::from(""),
            data: None,
            params: vec![],
            normal_children: vec![],
            param_child: Box::new(None),
            catch_all_child: Box::new(None),
        }
    }
}

impl<T> Node<T> {
    fn new_normal(segment: &str) -> Node<T> {
        Node {
            name: segment.to_string(),
            ..Node::default()
        }
    }

    fn new_param() -> Node<T> {
        Node {
            kind: NodeKind::Param,
            ..Node::default()
        }
    }

    fn new_cache_all() -> Node<T> {
        Node {
            kind: NodeKind::CatchAll,
            ..Node::default()
        }
    }

    fn child_index(&self, segment: &str) -> Option<usize> {
        if let Ok(i) = self.normal_children.binary_search_by(|n| {
            let name = &(n.name)[..];
            name.cmp(segment)
        }) {
            return Some(i);
        }
        None
    }

    fn will_conflit(&self, segment: &str) -> bool {
        if segment.starts_with(':') && self.catch_all_child.is_some() {
            return true;
        }
        if segment.starts_with('*') && self.param_child.is_some() {
            return true;
        }

        false
    }

    fn param_name(&self, segment: &str) -> Option<String> {
        if segment.starts_with(':') || segment.starts_with('*') {
            Some(String::from(&segment[1..]))
        } else {
            None
        }
    }

    fn add_segment(&mut self, segment: &str) -> Result<&mut Node<T>, Error> {
        if self.will_conflit(segment) {
            return Err(Error::RouteConflict);
        }

        if segment.starts_with(':') {
            return match *self.param_child {
                Some(ref mut n) => Ok(n),
                None => {
                    self.param_child = Box::new(Some(Node::new_param()));
                    match *self.param_child {
                        Some(ref mut n) => Ok(n),
                        None => panic!("impossible"),
                    }
                }
            };
        }

        if segment.starts_with('*') {
            return match *self.catch_all_child {
                Some(ref mut n) => return Ok(n),
                None => {
                    self.catch_all_child = Box::new(Some(Node::new_cache_all()));
                    match *self.catch_all_child {
                        Some(ref mut n) => Ok(n),
                        None => panic!("impossible"),
                    }
                }
            };
        }

        if self.child_index(segment).is_none() {
            self.normal_children.push(Node::new_normal(segment));
            self.normal_children.sort_by(|a, b| a.name.cmp(&b.name))
        }
        let idx = self.child_index(segment).unwrap();
        return Ok(&mut self.normal_children[idx]);
    }

    fn set_data(&mut self, data: T) {
        self.data = Some(data)
    }
}

#[derive(Debug)]
struct Match<T> {
    data: T,
    params: BTreeMap<String, String>,
}

struct Router<T> {
    root: Node<T>,
}

impl<T> Default for Router<T> {
    fn default() -> Router<T> {
        Router {
            root: Node::default(),
        }
    }
}

impl<T> Router<T> {
    fn is_route_valid(&self, path: &str) -> bool {
        if !path.starts_with('/') {
            return false;
        }

        if path.len() > 1 && path.ends_with('/') {
            return false;
        }

        if path.len() == 1 {
            return true;
        }

        let path = &path[1..];
        let mut checker = BTreeSet::new();
        let mut has_catch_all = false;
        for segment in path.split('/') {
            if segment.len() == 0 || has_catch_all {
                return false;
            }
            if segment.starts_with(':') || segment.starts_with('*') {
                if segment.len() == 1 {
                    return false;
                }
                let name = &segment[1..];
                if checker.contains(name) {
                    return false;
                }
                checker.insert(&segment[1..]);
            }

            if segment.starts_with('*') {
                has_catch_all = true
            }
        }

        return true;
    }
    pub fn add(&mut self, path: &str, data: T) -> Result<&mut T, Error> {
        if !self.is_route_valid(path) {
            return Err(Error::InvalidFormat);
        }

        let path = &path[1..];
        let mut last = &mut self.root;
        let mut params = vec![];
        for segment in path.split('/') {
            if segment.len() == 0 {
                break;
            }

            let rs = last.add_segment(segment);
            last = match rs {
                Ok(r) => {
                    match r.kind {
                        NodeKind::Param | NodeKind::CatchAll => {
                            params.push(r.param_name(segment).unwrap());
                        }
                        NodeKind::Static => (),
                    }
                    r
                }
                Err(err) => return Err(err),
            };
        }

        // refine codes here
        if params.len() > 0 && last.params.len() == 0 {
            last.params = params;
        } else if params != last.params {
            return Err(Error::RouteConflict);
        }

        last.set_data(data);
        match last.data {
            Some(ref mut d) => Ok(d),
            None => panic!("impossible"),
        }
    }

    pub fn recognize<'a>(&'a self, path: &str) -> Option<Match<&'a T>> {
        let path = {
            if path == "" {
                "/"
            } else {
                path
            }
        };

        if !path.starts_with('/') {
            return None;
        }

        let mut last = &self.root;
        let mut is_catching_all = false;
        let mut catch_all = String::from("");
        let mut values = vec![];
        let path = &path[1..];
        for segment in path.split('/') {
            if is_catching_all {
                catch_all.push('/');
                catch_all.push_str(segment);
                continue;
            }

            if segment.len() == 0 {
                continue;
            }

            if let Some(idx) = last.child_index(segment) {
                last = &last.normal_children[idx];
                continue;
            }

            if let Some(ref node) = *last.param_child {
                values.push(segment);
                last = node;
                continue;
            }

            if let Some(ref node) = *last.catch_all_child {
                is_catching_all = true;
                catch_all.push_str(segment);
                last = node;
                continue;
            }

            if segment.len() != 0 {
                return None; // miss
            }
        }

        if is_catching_all {
            values.push(catch_all.as_str())
        }

        match last.data {
            Some(ref data) => {
                let mut params = BTreeMap::<String, String>::new();
                for (k, v) in last.params.iter().zip(values) {
                    params.insert(k.clone(), String::from(v));
                }
                Some(Match { data, params })
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_router() {
        const ROUTES: [&'static str; 10] = [
            "/",
            "/users",
            "/users/:id",
            "/users/:id/:org",
            "/users/:user_id/repos",
            "/users/:user_id/repos/:id",
            "/users/:user_id/repos/:id/*any",
            "/about",
            "/about/us",
            "/:username",
        ];

        let mut router = Router::default();

        for (i, route) in ROUTES.iter().enumerate() {
            router.add(route, i);
        }

        let checks = vec![
            ("/", true, 0, vec![]),
            ("/users", true, 1, vec![]),
            ("/users/", true, 1, vec![]),
            ("/users/42", true, 2, vec![("id", "42")]),
            ("/users/四十二", true, 2, vec![("id", "四十二")]),
            ("/users/****", true, 2, vec![("id", "****")]),
            (
                "/users/42/ruster",
                true,
                3,
                vec![("id", "42"), ("org", "ruster")],
            ),
            ("/users/42/repos", true, 4, vec![("user_id", "42")]),
            ("/users/42/repos/", true, 4, vec![("user_id", "42")]),
            (
                "/users/42/repos/12",
                true,
                5,
                vec![("user_id", "42"), ("id", "12")],
            ),
            (
                "/users/42/repos/12/",
                true,
                5,
                vec![("user_id", "42"), ("id", "12")],
            ),
            (
                "/users/42/repos/12/x",
                true,
                6,
                vec![("user_id", "42"), ("id", "12"), ("any", "x")],
            ),
            (
                "/users/42/repos/12/x/y/z",
                true,
                6,
                vec![("user_id", "42"), ("id", "12"), ("any", "x/y/z")],
            ),
            (
                "/users/42/repos/12/x/y/z/",
                true,
                6,
                vec![("user_id", "42"), ("id", "12"), ("any", "x/y/z/")],
            ),
            (
                "/users/42/repos/12/x/山口山/z",
                true,
                6,
                vec![("user_id", "42"), ("id", "12"), ("any", "x/山口山/z")],
            ),
            ("/about", true, 7, vec![]),
            ("/about/us", true, 8, vec![]),
            ("/somebody", true, 9, vec![("username", "somebody")]),
            ("/某人", true, 9, vec![("username", "某人")]),
            ("/某人/", true, 9, vec![("username", "某人")]),
            ("/somebody/", true, 9, vec![("username", "somebody")]),
            ("/about/", true, 7, vec![]),
            ("/about/what", false, 0, vec![]),
            ("/somebody/what", false, 0, vec![]),
            ("/某人/what", false, 0, vec![]),
            ("/users/42/ruster/12", false, 0, vec![]),
            ("/users/42/ruster/12/a", false, 0, vec![]),
        ];

        for (path, exist, val, param) in checks.iter() {
            if *exist {
                let m = router.recognize(*path).unwrap();
                assert_eq!(m.data, val);
                for (k, v) in param {
                    match m.params.get(*k) {
                        Some(ref rv) => assert_eq!(v, rv),
                        None => panic!("{} not found", k),
                    }
                }
            } else {
                assert!(router.recognize(*path).is_none());
            }
        }
    }

    #[test]
    fn invalid_routes() {
        let checks = vec![
            ("/dup/:id/:id", false, vec![]),
            ("/double_slash//a", false, vec![]),
            ("/double_slash///a", false, vec![]),
            ("/trailing_slash/", false, vec![]),
            ("/empty_param/:", false, vec![]),
            ("/empty_param/:/a", false, vec![]),
            ("/empty_catch_all/*", false, vec![]),
            ("/different_param_name/:a", true, vec!["a"]),
            ("/different_param_name/:b", false, vec![]),
            ("/different_param_name/:b/:c", true, vec!["b", "c"]),
            ("/different_param_name/:a/:d", false, vec![]),
            ("/different_param_name/:a/:d/*e", true, vec!["a", "d", "e"]),
            ("/catch_all_not_the_last/*a/extra", false, vec![]),
        ];

        let mut router = Router::default();

        for (route, valid, keys) in checks.iter() {
            let rs = router.add(*route, 1);
            if *valid {
                assert_eq!(*rs.unwrap(), 1);
                match router.recognize(*route) {
                    None => panic!("failed to recognize {}", *route),
                    Some(Match {data: _, params}) => {
                        for k in keys.iter() {
                            assert!(params.get(*k).is_some(), "miss capturing param: {}", *k)
                        }
                    }
                }
            } else {
                assert!(rs.is_err());
            }
        }
    }
}
