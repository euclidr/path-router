extern crate criterion;
extern crate actix_router;
extern crate path_router;
extern crate path_table;
extern crate path_tree;
extern crate route_recognizer;

use criterion::{Benchmark, Criterion};
use path_router::Router;

use actix_router::{Path as ActixPath, Router as ActixRouter};
use criterion::*;
use path_table::PathTable;
use path_tree::PathTree;
use route_recognizer::Router as RRRouter;

#[path = "../tests/fixtures/github.rs"]
mod github;

use github::*;

fn bench_build_static_route(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "build_static_route",
        |b, &route| {
            b.iter(|| {
                let mut router = Router::default();
                router.add(route, 1).unwrap();
            })
        },
        vec!["/", "/aaa/bbb", "/aaa/bbb/ccc/ddd"],
    );
}

fn bench_build_param_route(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "build_param_route",
        |b, &route| {
            b.iter(|| {
                let mut router = Router::default();
                router.add(route, 1).unwrap();
            })
        },
        vec!["/:aaa", "/:aaa/:bbb", "/:aaa/:bbb/:ccc/:ddd"],
    );
}

fn bench_build_catchall_route(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "build_catchall_route",
        |b, &route| {
            b.iter(|| {
                let mut router = Router::default();
                router.add(route, 1).unwrap();
            })
        },
        vec!["/*aaa", "/aaa/*bbb"],
    );
}

fn bench_recognize(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "build_recognize",
        |b, &path| {
            let router = {
                let mut router = Box::new(Router::default());
                router.add("/", 1).unwrap();
                router.add("/user", 1).unwrap();
                router.add("/user/:id", 1).unwrap();
                router.add("/user/:id/*any", 1).unwrap();
                router
            };
            b.iter(move || {
                router.recognize(path);
            })
        },
        vec!["/", "/user", "/user/123", "/user/123/repos", "/users"],
    );
}

// inspired by path-tree https://github.com/trek-rs/path-tree/blob/master/benches/routers.rs
fn bench_path_insert(c: &mut Criterion) {
    c.bench(
        "path_insert",
        Benchmark::new("path_tree_insert", |b| {
            let mut tree: PathTree<usize> = PathTree::new();
            b.iter(|| {
                for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                    tree.insert(r, i);
                }
            })
        })
        .with_function("route_recognizer_add", |b| {
            let mut router = RRRouter::<usize>::new();
            b.iter(|| {
                for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                    router.add(r, i);
                }
            })
        })
        .with_function("path_table_setup", |b| {
            let mut table: PathTable<usize> = PathTable::new();
            b.iter(|| {
                for (i, r) in ROUTES_WITH_BRACES.iter().enumerate() {
                    *table.setup(r) = i;
                }
            })
        })
        .with_function("actix_router_path", |b| {
            let mut router = ActixRouter::<usize>::build();
            b.iter(|| {
                for (i, r) in ROUTES_WITH_BRACES.iter().enumerate() {
                    router.path(r, i);
                }
            })
        })
        .with_function("path_router_add", |b| {
            let mut router = Router::<usize>::default();
            b.iter(|| {
                for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                    router.add(r, i).unwrap();
                }
            })
        })
        .sample_size(50),
    );
}

// inspired by path-tree https://github.com/trek-rs/path-tree/blob/master/benches/routers.rs
fn bench_path_find(c: &mut Criterion) {
    c.bench(
        "path_find",
        Benchmark::new("path_tree_find", |b| {
            let mut tree: PathTree<usize> = PathTree::new();
            for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                tree.insert(r, i);
            }
            b.iter(|| {
                for (i, r) in ROUTES_URLS.iter().enumerate() {
                    let n = tree.find(r).unwrap();
                    assert_eq!(*n.0, i);
                }
            })
        })
        .with_function("route_recognizer_recognize", |b| {
            let mut router = RRRouter::<usize>::new();
            for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                router.add(r, i);
            }
            b.iter(|| {
                for (i, r) in ROUTES_URLS.iter().enumerate() {
                    let n = router.recognize(r).unwrap();
                    assert_eq!(*n.handler, i);
                }
            })
        })
        .with_function("path_table_route", |b| {
            let mut table: PathTable<usize> = PathTable::new();
            for (i, r) in ROUTES_WITH_BRACES.iter().enumerate() {
                *table.setup(r) = i;
            }
            b.iter(|| {
                for (i, r) in ROUTES_URLS.iter().enumerate() {
                    let n = table.route(r).unwrap();
                    assert_eq!(*n.0, i);
                }
            })
        })
        .with_function("actix_router_recognize", |b| {
            let mut router = ActixRouter::<usize>::build();
            for (i, r) in ROUTES_WITH_BRACES.iter().enumerate() {
                router.path(r, i);
            }
            let router = router.finish();
            b.iter(|| {
                for (i, r) in ROUTES_URLS.iter().enumerate() {
                    let mut path = ActixPath::new(*r);
                    let n = router.recognize(&mut path).unwrap();
                    assert_eq!(*n.0, i);
                }
            })
        })
        .with_function("path_router_recognize", |b| {
            let mut router = Router::<usize>::default();
            for (i, r) in ROUTES_WITH_COLON.iter().enumerate() {
                router.add(r, i).unwrap();
            }
            b.iter(|| {
                for (i, r) in ROUTES_URLS.iter().enumerate() {
                    let m = router.recognize(r).unwrap();
                    assert_eq!(m.data, &i);
                }
            })
        })
        .sample_size(50),
    );
}

criterion_group!(
    benches,
    bench_build_static_route,
    bench_build_param_route,
    bench_build_catchall_route,
    bench_recognize,
    bench_path_insert,
    bench_path_find
);
criterion_main!(benches);
