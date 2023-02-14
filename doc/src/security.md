# Security


## Dispatcher Security

Dispatcher is the component responsible for executing the customer provided webhooks. There are two main attack vectors for the dispatcher.

### 1. Private IPs

Given that the customer has full control on both the endpoint and payload, they can leverage the fact that the dispatcher runs in our infra to call private endpoints that should have not been accessible otherwise. E.g. Calling the private scheduler gRPC service by forging an endppint with localhost:8888 as the url. To mitigate this threat, we will:
1. Only accept endpoints that resolve to publicly routable IPs. This needs to happen directly before invoking the webook. It's not enough to validate this at Trigger creation time.
2. The dispatcher should never follow redirects (3XX). Redirects can be used as a way to get around the first mitigation given that we only validate the very first endpoint and nothing else. There are ways to hook into redirects and repeat the validations there, but for now, we're just not going to follow redirects.
3. In prod, the dispatcher will be invoking the webhooks through an HTTP proxy that's hosted in an isolated virtual private network separate from our infra's VPN. Firewall rules will be setup in a way to restrict incoming connections to this network to be only allowed from inside our infra.


### 2. Exhausting Resources

The customer controls the destination server. This can allow it to respond slowly for example wasting server resources, or even send huge payloads that can OOM the service. To mitigate that we will:
1. Set a timeout on every request to the destination service.
2. After receiving the HTTP response, we will decide whether we'll read the payload or not based on its content length.
