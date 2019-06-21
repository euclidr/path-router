use std::collections::BTreeMap;
use std::default::Default;

enum NodeKind {
    Normal,
    Param,
    CatchAll,
}

impl Default for NodeKind {
    fn default() -> NodeKind {
        NodeKind::Normal
    }
}

struct Node<T> {
    kind: NodeKind,
    name: String,
    data: Option<T>,
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

    fn new_param(segment: &str) -> Node<T> {
        Node {
            kind: NodeKind::Param,
            name: segment.to_string(),
            ..Node::default()
        }
    }

    fn new_cache_all(segment: &str) -> Node<T> {
        Node {
            kind: NodeKind::CatchAll,
            name: segment.to_string(),
            ..Node::default()
        }
    }

    // fn child_from_segment(&mut self, segment: &str) -> Option<&mut Node<T>> {
    //     unimplemented!()
    // }

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
        if segment.starts_with(':') {
            if self.catch_all_child.is_some() {
                return true;
            }
            let segment = &segment[1..];
            return match *self.param_child {
                Some(ref n) if n.name == segment => false,
                None => false,
                _ => true,
            };
        } else if segment.starts_with('*') {
            if self.param_child.is_some() {
                return true;
            }
            let segment = &segment[1..];
            return match *self.catch_all_child {
                Some(ref n) if n.name == segment => false,
                None => false,
                _ => true,
            };
        }

        false
    }

    fn add_segment(&mut self, segment: &str) -> Result<&mut Node<T>, String> {
        if self.will_conflit(segment) {
            return Err("conflit".to_string());
        }

        if segment.starts_with(':') {
            let segment = &segment[1..];
            match *self.param_child {
                Some(ref mut n) => return Ok(n),
                None => {
                    self.param_child = Box::new(Some(Node::new_param(segment)));
                    if let Some(ref mut n) = *self.param_child {
                        return Ok(n);
                    }
                }
            }
        } else if segment.starts_with('*') {
            let segment = &segment[1..];
            match *self.catch_all_child {
                Some(ref mut n) => return Ok(n),
                None => {
                    self.catch_all_child = Box::new(Some(Node::new_cache_all(segment)));
                    if let Some(ref mut n) = *self.catch_all_child {
                        return Ok(n);
                    }
                }
            }
        } else {
            if self.child_index(segment).is_none() {
                self.normal_children.push(Node::new_normal(segment));
                self.normal_children.sort_by(|a, b| a.name.cmp(&b.name))
            }
            let idx = self.child_index(segment).unwrap();
            return Ok(&mut self.normal_children[idx]);
        }

        Err("impossible".to_string())
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
    pub fn add(&mut self, path: &str, data: T) {
        if !path.starts_with('/') {
            panic!("path schema must start with /");
        }

        if path.len() > 1 && path.ends_with('/') {
            panic!("path schema must not end with /");
        }

        if path.contains("//") {
            panic!("invalid path schema");
        }

        let path = &path[1..];
        let mut last = &mut self.root;
        for segment in path.split('/') {
            if segment.len() == 0 {
                break;
            }

            let rs = last.add_segment(segment);
            last = match rs {
                Ok(r) => r,
                Err(_) => return,
            };
        }

        last.set_data(data);
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
        let mut params = BTreeMap::<String, String>::new();
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
                params.insert(node.name.clone(), String::from(segment));
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
            params.insert(last.name.clone(), catch_all);
        }

        match last.data {
            Some(ref data) => Some(Match { data, params }),
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
            "/users/:user_id",
            "/users/:user_id/:org",
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
            ("/users/42", true, 2, vec![("user_id", "42")]),
            ("/users/四十二", true, 2, vec![("user_id", "四十二")]),
            ("/users/****", true, 2, vec![("user_id", "****")]),
            ("/users/42/ruster", true, 3, vec![("user_id", "42"), ("org", "ruster")]),
            ("/users/42/repos", true, 4, vec![("user_id", "42")]),
            ("/users/42/repos/", true, 4, vec![("user_id", "42")]),
            ("/users/42/repos/12", true, 5, vec![("user_id", "42"), ("id", "12")]),
            ("/users/42/repos/12/", true, 5, vec![("user_id", "42"), ("id", "12")]),
            ("/users/42/repos/12/x", true, 6, vec![("user_id", "42"), ("id", "12"), ("any", "x")]),
            ("/users/42/repos/12/x/y/z", true, 6, vec![("user_id", "42"), ("id", "12"), ("any", "x/y/z")]),
            ("/users/42/repos/12/x/y/z/", true, 6, vec![("user_id", "42"), ("id", "12"), ("any", "x/y/z/")]),
            ("/users/42/repos/12/x/山口山/z", true, 6, vec![("user_id", "42"), ("id", "12"), ("any", "x/山口山/z")]),
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
}
