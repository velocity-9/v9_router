# v9_router
Routing Application to handle incoming REST calls

## General Outline
- Start server to receive REST calls
- Upon receiving request
  - Parse request into the different pieces (user, repo, method)
  - Query all the servers to figure out which one is the best
  - Make a request to that server including all info

## MVP
- Receive request
- Break it down
- Send to the one server
