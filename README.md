# path router

a simple path router for HTTP server

### Features

* support name parameters like `:name` and CatchAll parameters like `*any`
* support creating sub routers

### Limitation(current)

* `*any` must be the last segment in route

### Usage

```
extern crate path_router;
use path_router::Router;

let mut router = Router::default();
router.add("/a/path", 1).unwrap();
router.add("/user/:id/repos", 2).unwrap();
router.add("/user/:user_id/repos/:id", 3).unwrap();
router.add("/list/*animals", 4).unwrap();

assert_eq!(*router.recognize("/a/path").unwrap().data, 1);
assert_eq!(*router.recognize("/user/100/repos").unwrap().data, 2);
assert_eq!(*router.recognize("/user/100/repos/1").unwrap().data, 3);
assert_eq!(*router.recognize("/list/*animals").unwrap().data, 4)
```

### Examples

Please read [examples/user](examples/user.rs)


