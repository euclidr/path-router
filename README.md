# path router

a simple path routing table

TODO:

1. support different wildcard name like ✔

    ```
    /user/:id
    /user/:user_id/repo/:id
    ```

    but

    ```
    /user/:id
    /user/:user_id
    ```

    should not be permitted

2. prevent same wildcard name in a single route: ✔

    ```
    /user/:id/repo/:id // not allowed
    /:a/*a // not allowed
    ```

3. clean route before recognizing

    ```
    //a/bb/a/..// => /a/bb
    ```

4. route can be chained

    ```
    admin = route.add_base("/admin")
    admin.add("/", endpoint)
    dashboard = admin.add_base("/dashboard")
    dashboard.add("/temprature, endpoint)
    ```

5. benchmark

6. middleware?

7. regex?
