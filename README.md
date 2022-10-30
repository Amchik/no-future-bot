<div align="center">
  <h1>🌧 no-future-bot</h1>
  <p>API server and telegram bot for manage channel posting and stole posts from twitter</p>
  <b>work in progress</b>
</div>

## Building and running ##

```console
$ cargo build
$ cargo run -- -t "TELEGRAM_TOKEN" # or...
$ TELEGRAM_TOKEN=$secret cargo run
```

## Usage ##
Create telegram bot via [@BotFather](https://t.me/botfather),
  go to `Bot Settings` and set `Menu Button`.

For now, this bot is useless.

## API Endpoints ##
API may return 3 types of response:

1. Result. HTTP status is 200 and json schema is `{"result": { /* object */ } }`
2. No result. HTTP status is 204 and no data returned.
3. Error. HTTP status in 400..599 and json schema is `{ "error_code": 400, "error_description": "Error Description" }`

### Authorization ###
In many endpoints you need to provide WebApp `initData` in `X-InitData` header.
More info in telegram documentation: [Initializing Web Apps](https://core.telegram.org/bots/webapps#initializing-web-apps).
Session is valid for **15 minutes**. If `initData` is invalid or expired request will fail with `403: Forbidden`.

### Models ###

<table>
<tr>
<td>

```ts
interface User {
  id: int,
  channel: int | null,
  power_level: int
}
```
</td>
<td>

```ts
interface ChannelData {
  // Integer or string that
  // starts with '@'
  channel_id: string
}
```
</td>
</tr>
</table>

### User endpoints ###
File: [`src/routes/user.rs`](src/routes/user.rs).

| Method | Path       | Description              | Body Type   | Return Type |
|--------|------------|--------------------------|-------------|-------------|
| GET    | `/user`    | Returns self user object |             | User        |
| POST   | `/user`    | Link channel             | ChannelData | User        |
| DELETE | `/user`    | Delete self account      |             | Nothing     |
