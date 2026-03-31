# Matthew API Definition

This file describes the API that Matthew serves.

## Invoke counting an item

```http
POST /api/count
Authorication: Bearer [TOKEN]

{
  "repo":"user/repo",
  "callback":"https://ferris.love/api/countCallback?repo=user%2Frepo&token=xxxxxxx"
}
```

### Returns

```json
{
  "success":true
}
```

Or

```json
{
  "success":false
}
```

if Token is invalid.

### Callback

Matthew will POST to the callback URL with the following headers:
- `User-Agent: Matthew`
- `Content-Type: application/json`
- `X-Signature-256: sha256=<hex-encoded HMAC-SHA256 of body using callback_secret>`

```http
POST [callback-url]

{
  "repo":"user/repo",
  "status":"pending",
  "data":null,
  "error":null,
}

{
  "repo":"user/repo",
  "status":"done",
  "data":{
    "counts":[Counts],
    "lorc":0, // Lines of Rust code
  },
  "error":null
}

{
  "repo":"user/repo",
  "status":"error",
  "data":null,
  "error":"Repository size exceed the maxium capacity"
}
```
