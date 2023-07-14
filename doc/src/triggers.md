# Triggers

Endpoint: GET `/v1/triggers/:id`

```javascript
{
  "id": "trg_2345923hbdhbfiqwerbwer",
  "owner_id": "acc_2342495239423423423",
  "reference_id": "user-supplied-id",
  // optional
  "name": "Remind user to finish sign-up",
  "description": "Something something something",
  "created_at": "1997-07-16T19:20:30.45Z",
  "action": [
      {
        "webhook": {
          "url": "https://google.com/something",
          "http_method": "https://google.com/something",
          "timeout_s": 5.0,
          "retry": {
            "simple_retry": {
              "max_num_attempts": 10,
              "delay_s": 10.0
            }
          }
        }
      }
  ],
  "payload": {
    "body": "anything",
    "content_type": "application/json; charset=utf-8",
    "headers": {
      "Something": "5"
    }
  },
  "schedule": {
    "recurring": {
      "cron": "0 * * * *", // mut. exl. with run_at
      "cron_timezone": "Europe/London",
      // [optional, no limit by default or if set to 0]
      "cron_events_limit": "1",
    },
    // OR
    "run_at": [
      // can have uP to 100 points, has timezone in iso 8601 format.
      "1997-07-16T19:20:30.45Z",
      // Supports ISO-8601 durations as well (e.g. PT5M)
      "1997-07-16T19:20:30.45Z",
    ],
  },
  "status": "active" | "expired" | "paused" | "cancelled",
}

```

## Creating a trigger

```bash
# POST /v1/triggers

/**
 *  curl https://api.cronback.dev/v1/triggers \
 *  -H "Authorization: Bearer <token>"\
 *  -d name="something something"
 *  -d endpoint="https://example.com:9000/myendpoint"
 *  -d payload="{\"key\": \"value\"}"
 *  -d content_type="application/json; charset=utf8"
 *  -d cron="*/2 * * * *" # every 2 minutes
*/

# Response
200 OK

{
  "id": "trg_2345923hbdhbfiqwerbwer",
  "object": "trigger",
  "name": "name",
  "created_at": "2023-02-02T21:39+00:00",
  "http_method": "POST",
  "endpoint": "https://example.com:9000/myendpoint",
  "payload": "{\"key\": \"value\"}",
  "content_type": "application/json; charset=utf-8",
  "cron": "*/2 * * * *",
  "cron_timezone": "Etc/UTC",
  "status": "active"
}
```
