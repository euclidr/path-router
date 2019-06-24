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

/// match result of a path
#[derive(Debug)]
pub struct Match<T> {
    /// data set in the route
    pub data: T,
    /// extracted parameters from the path
    pub params: BTreeMap<String, String>,
}

/// A generic path router
///
/// it can route to exact path like `/a/path`
/// it use prefix `:` to catch parameters in path:
/// with route `/user/:id/repos` we can get `123` from path `/user/123/repos`
/// if use prefix `*` to catch all the rest of the path:
/// with route `/list/*animals` we can get `chicken/duck` from path
/// `/list/chiken/duct`
///
/// # Example
///
/// ```
/// use path_router::Router;
/// let mut router = Router::default();
/// router.add("/a/path", 1).unwrap();
/// router.add("/user/:id/repos", 2).unwrap();
/// router.add("/list/*animals", 3).unwrap();
///
/// assert_eq!(*router.recognize("/a/path").unwrap().data, 1);
/// assert_eq!(*router.recognize("/user/:id/repos").unwrap().data, 2);
/// assert_eq!(*router.recognize("/list/*animals").unwrap().data, 3);
/// ```
pub struct Router<T> {
    kind: NodeKind,
    text: String, // text of static node, empty string if it's wildcard node
    data: Option<T>,
    params: Vec<String>, // param or catchall keys of the route, order by their occurrences
    static_children: Vec<Router<T>>,
    param_child: Box<Option<Router<T>>>,
    catch_all_child: Box<Option<Router<T>>>,
}

impl<T> Default for Router<T> {
    fn default() -> Router<T> {
        Router::<T> {
            kind: NodeKind::default(),
            text: String::from(""),
            data: None,
            params: vec![],
            static_children: vec![],
            param_child: Box::new(None),
            catch_all_child: Box::new(None),
        }
    }
}

// impl<T> std::fmt::Debug for Router<T> {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         unimplemented!()
//     }
// }

// Router as node
impl<T> Router<T> {
    pub fn new() -> Router<T> {
        Router::default()
    }

    fn new_static_node(segment: &str) -> Router<T> {
        Router {
            text: segment.to_string(),
            ..Router::default()
        }
    }

    fn new_param_node() -> Router<T> {
        Router {
            kind: NodeKind::Param,
            ..Router::default()
        }
    }

    fn new_cache_all_node() -> Router<T> {
        Router {
            kind: NodeKind::CatchAll,
            ..Router::default()
        }
    }

    fn child_index(&self, segment: &str) -> Option<usize> {
        if let Ok(i) = self.static_children.binary_search_by(|n| {
            let name = &(n.text)[..];
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

    fn add_segment(&mut self, segment: &str) -> Result<&mut Router<T>, Error> {
        if self.will_conflit(segment) {
            return Err(Error::RouteConflict);
        }

        if segment.starts_with(':') {
            return match *self.param_child {
                Some(ref mut n) => Ok(n),
                None => {
                    self.param_child = Box::new(Some(Router::new_param_node()));
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
                    self.catch_all_child = Box::new(Some(Router::new_cache_all_node()));
                    match *self.catch_all_child {
                        Some(ref mut n) => Ok(n),
                        None => panic!("impossible"),
                    }
                }
            };
        }

        if self.child_index(segment).is_none() {
            self.static_children.push(Router::new_static_node(segment));
            self.static_children.sort_by(|a, b| a.text.cmp(&b.text))
        }
        let idx = self.child_index(segment).unwrap();
        return Ok(&mut self.static_children[idx]);
    }

    fn set_data(&mut self, data: T) {
        self.data = Some(data)
    }
}

/// Router as router
impl<T> Router<T> {
    /// set a route with data
    ///
    /// a route must start with '/' and have no trailing '/'
    /// empty text between '/' is not allowed
    /// empty parameter name or empty catchall name like "/a/:/b" or "/a/*" is not allowed
    /// catchall must be the last segment if any
    /// parameter namse and catchall name must not be duplicated
    pub fn add(&mut self, route: &str, data: T) -> Result<&mut T, Error> {
        if !self.is_valid_route(route) {
            return Err(Error::InvalidFormat);
        }

        let path = &route[1..];
        let mut last = self;
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

    /// create a sub route from current route
    ///
    /// route must be static, parameters and catch all are not allowed
    pub fn sub_route(&mut self, route: &str) -> Result<&mut Router<T>, Error> {
        if !self.is_valid_base(route) {
            return Err(Error::InvalidFormat);
        }

        let path = &route[1..];
        let mut last = self;
        for segment in path.split('/') {
            if segment.len() == 0 {
                break;
            }

            let rs = last.add_segment(segment);
            last = rs.unwrap();
        }

        Ok(last)
    }

    /// recognize a path
    ///
    /// path must start with '/'
    /// path should not have segments like '..', '.'
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

        let mut last = self;
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
                last = &last.static_children[idx];
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

    pub fn list_routes(&self) -> Vec<String> {
        self.list_sub_routes(&vec![])
    }

    fn combine_route_parts(&self, parts: &Vec<String>, params: &Vec<String>) -> String {
        if parts.len() == 1 && parts[0] == "" {
            return String::from("/");
        }

        let mut i = 0;
        let mut parts = parts.clone();
        for part in parts.iter_mut() {
            if part == ":" || part == "*" {
                *part = format!("{}{}", part, params[i]);
                i = i + 1;
                continue;
            }
        }
        parts.join("/")
    }

    fn list_sub_routes(&self, pre: &Vec<String>) -> Vec<String> {
        let mut result = vec![];
        let mut cur = pre.clone();
        match self.kind {
            NodeKind::Static => cur.push(self.text.clone()),
            NodeKind::Param => cur.push(String::from(":")),
            NodeKind::CatchAll => cur.push(String::from("*")),
        }

        if self.data.is_some() {
            result.push(self.combine_route_parts(&cur, &self.params))
        }

        for node in self.static_children.iter() {
            result.append(&mut node.list_sub_routes(&cur));
        }

        if let Some(ref node) = *self.param_child {
            result.append(&mut node.list_sub_routes(&cur));
        }

        if let Some(ref node) = *self.catch_all_child {
            result.append(&mut node.list_sub_routes(&cur));
        }

        result
    }

    fn is_route_in_good_shape(&self, route: &str) -> bool {
        if !route.starts_with('/') {
            return false;
        }

        if route.len() > 1 && route.ends_with('/') {
            return false;
        }

        return true;
    }

    fn is_valid_route(&self, route: &str) -> bool {
        if !self.is_route_in_good_shape(route) {
            return false;
        }

        if route.len() == 1 {
            return true;
        }

        let path = &route[1..];
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

    fn is_valid_base(&self, route: &str) -> bool {
        if !self.is_route_in_good_shape(route) {
            return false;
        }

        if route.len() == 1 {
            return true;
        }

        let path = &route[1..];
        for segment in path.split('/') {
            if segment.len() == 0 {
                return false;
            }
            if segment.starts_with(':') || segment.starts_with('*') {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_simple_router(router: &mut Router<usize>) {
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

        for (i, route) in ROUTES.iter().enumerate() {
            router.add(route, i).unwrap();
        }
    }

    fn check_with_base(router: &Router<usize>, base: &str) {
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
            let path_string = format!("{}{}", base, *path);
            if *exist {
                let m = router.recognize(&path_string).unwrap();
                assert_eq!(m.data, val);
                for (k, v) in param {
                    match m.params.get(*k) {
                        Some(ref rv) => assert_eq!(v, rv),
                        None => panic!("{} not found", k),
                    }
                }
            } else {
                assert!(router.recognize(&path_string).is_none());
            }
        }
    }

    #[test]
    fn simple_router() {
        let mut router = Router::default();
        build_simple_router(&mut router);
        check_with_base(&router, "");
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
                    Some(Match { data: _, params }) => {
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

    #[test]
    fn base_route() {
        let mut router = Router::default();
        build_simple_router(&mut router);
        {
            let admin = router.sub_route("/admin").unwrap();
            build_simple_router(admin);
            {
                let console = admin.sub_route("/console").unwrap();
                build_simple_router(console);
                check_with_base(console, "");
            }
            check_with_base(admin, "");
            check_with_base(admin, "/console");
        }
        check_with_base(&router, "");
        check_with_base(&router, "/admin");
        check_with_base(&router, "/admin/console");
    }

    #[test]
    fn display_routes() {
        let mut router = Router::default();
        build_simple_router(&mut router);
        println!("{:?}", router.list_routes())
    }
}
