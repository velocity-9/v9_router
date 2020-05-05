# v9_router
[![CircleCI](https://circleci.com/gh/velocity-9/v9_router.svg?style=svg)](https://circleci.com/gh/velocity-9/v9_router)

Routing Application to handle incoming REST calls

## Running the Router
We use the following command line to run the router (obviously your own worker URLs must be provided): 
```
sudo sh -c "export V9_WORKERS='http://v9_w1.example.com;http://v9_w2.example.com';cargo run --release"
```
This will serve HTTP requests on port 80.

To get HTTPS support, use this command line:
```
sudo sh -c "export V9_WORKERS='http://v9_w1.example.com;http://v9_w2.example.com';cargo run --release -- --development"
```
Which sets the router port to be 8080. 
Then setup an NGINX reverse proxy with HTTPs support to tunnel encrypted traffic to 8080.
(We used this [guide](https://medium.com/@mightywomble/how-to-set-up-nginx-reverse-proxy-with-lets-encrypt-8ef3fd6b79e5))
