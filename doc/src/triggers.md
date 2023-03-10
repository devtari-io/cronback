# Triggers

Endpoint: `/v1/triggers`


```javascript
{
  "id": "trg_2345923hbdhbfiqwerbwer",
  "object": "trigger",
  "reference_id": "user-supplied-id",
  // optional
  "name": "Remind user to finish sign-up",
  "description": "Something something something", 
  "created_at": "1997-07-16T19:20:30.45Z",
  "emit": {
    "webhook": {
      "url": "https://google.com/something",
      "http_method": "https://google.com/something",
      "content_type": "application/json; charset=utf-8",
      "timeout_s": 5,
    },
  },
  "payload": "anything",
  "headers": {
    "Something": "5"
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
    // can have up to 100 points, has timezone in iso 8601 format.
    "1997-07-16T19:20:30.45Z",
    // Supports ISO-8601 durations as well (e.g. PT5M)
    "1997-07-16T19:20:30.45Z",
    ],
  },
  "status": "active" | "expired" | "paused" | "canceled",
  "last_event_details": {  // optional
    "id": "evt_89425729345bbwfywerxxx",
    "status": "succeeded" | "failed".
    "finished_at": "1997-07-16T19:20:30.45Z"
  },
  // max_in_flight
  "on_success": {
    "notifications": ["slack"],
    // 3 auto cancel cron if last 3 events succeeded [optional, never auto cancel]
    "auto_cancel_after": "3", 
  },
  "event_retry_policy": {
    // null means no retries, event will be marked failed.
    "policy": null | "exponential" | "simple",  
    "limit":"5", 
    "delay_s": "2",
    "max_delay_s": "2", // only if exponential
    "notifications": ["slack"],
  },
  "on_failure": {
    "notifications": ["slack"],
    // 3 auto cancel cron if last 3 events failed
    "auto_cancel_after": "3", 
  },
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

