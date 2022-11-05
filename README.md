<div align="center">
  <h1>🌧 no-future-bot</h1>
  <p>API server and telegram bot for manage channel posting and stole posts from twitter</p>
  <b>work in progress</b>
</div>

## Building and running ##

```console
$ export WEBAPP_URL=https://...
$ cargo build [--release]
$ cargo run -- -t "TWITTER_TOKEN" -b "TELEGRAM_TOKEN" # or...
$ TELEGRAM_TOKEN=$secret TWITTER_TOKEN=... cargo run
```

`dotenv` budget version protip: `while read n; do eval export "$n"; done < .env`

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
interface Author {
  id: int, // internal
  platform_id: int // twitter
  name: string,
  username: string,
  avatar_url: string | null
}
```
</td>
<td>

```ts
interface Post {
  id: int, // internal
  platform_id: int // twitter
  author_id: int, // internal
  text: string,
  source_url: string,
  source_text: string
}
```
</td>
</tr>
<tr>
<td>

```ts
interface PostMedia {
  id: int,
  post_id: int, // internal
  media_type: "photo" | "video",
  media_url: string
}
```
</td>
<td>

```ts
interface PostData {
  post: Post,
  media: PostMedia[]
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
<tr>
<td>

```ts
interface FeedElement {
  post: Post,
  media: PostMedia[],
  author: Author
}
```
</td>
<td>

```ts
// One of FeedRead, FeedSubscribe
// and FeedUnsubscribe 
interface FeedUpdateData {}
interface FeedRead : FeedUpdateData {
  readUnder: int
}
```
</td>
<td>

```ts
interface FeedSubscribe : FeedUpdateData {
  subscribe: string // see ':id' in
}                   // author endpoints
interface FeedUnsubscribe : FeedUpdateData {
  unsubscribe: string
}
```
</td>
</tr>
<tr>
<td>

```ts
interface ScheduledPost {
  id: int,
  user_id: int,
  media_ids: string, // ids split by ','
  post_text: string,
  post_source: string,
  post_source_url: string
} // NOTE: it may be changed in future
```
</td>
<td>

```ts
interface CreateScheduledPost {
  post_id: int,
  post_text?: string,
  exclude_media?: int[]
}
```
</td>
<td>

```ts
interface ScheduledFeedElement {
  post: ScheduledPost,
  media: PostMedia[]
}
```
</td>
</tr>
</table>

### User endpoints ###
File: [`src/routes/user.rs`](src/routes/user.rs).

| Method | Path              | Description               | Body Type     | Return Type |
|--------|-------------------|---------------------------|---------------|-------------|
| GET    | `/user`           | Returns self user object  |               | `User`      |
| POST   | `/user`           | Link channel              | `ChannelData` | `User`      |
| DELETE | `/user`           | Delete self account       |               | Nothing     |
| GET    | `/user/following` | Returns following authors |               | `Author[]`  |

### Author endpoints ###
File: [`src/routes/author.rs`](src/routes/author.rs).

- `:id` has type `string` if it username, positive `int` if it platform id
  or negative `int` if it internal id.

| Method | Path                | Description               | Return Type  |
|--------|---------------------|---------------------------|--------------|
| GET    | `/author`           | Get all known authors     | `Author[]`   |
| GET    | `/author/:id`       | Get author object         | `Author`     |
| PUT*   | `/author/:id`       | Create (or update) author | `Author`     |
| GET    | `/author/:id/posts` | Returns self user object  | `PostData[]` |

\* `:id` cannot be internal id here

### Feed endpoints ###
File: [`src/routes/feed.rs`](src/routes/feed.rs).
| Method | Path                  | Description                         | Body Type             | Return Type              |
|--------|-----------------------|-------------------------------------|-----------------------|--------------------------|
| GET    | `/feed`               | Returns feed                        |                       | `FeedElement[]`          |
| PATCH  | `/feed`               | Modify ([un]subscribe, read) feed   | `FeedUpdateData`      | Nothing                  |
| GET    | `/feed/scheduled`     | Returns scheduled feed              |                       | `ScheduledFeedElement[]` |
| PUT    | `/feed/scheduled`     | Create scheduled post               | `CreateScheduledPost` | `ScheduledPost`          |
| DELETE | `/feed/scheduled/:id` | Delete scheduled post               |                       | Nothing                  |
