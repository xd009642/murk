# Architecture

Plans: 
* Use tower to handle request batching and statistic gathering
* mun to allow user scripting if they want to log extra statistics (thinking
about RTF for me)
* Plot generation and raw stat dumps
* Request cycling
* Suitable for accurate measuring (can we learn from wrk-2)

so as well as calculating the response time for a request we should measure
the wait time to actually start sending that request

## Prior Art 

* Inspired by wrk with it's lua scripting but trying to do it in a language
that is better than lua with neglible overhead
* wrk-2 is better for latency measurements apparently so learn from that
* Worth seeing what jmeter does 
* Rewrk as a rust load balancer 
* Locust I guess, or goose as a rust engine

## Plan for mun API

### Prior art (wrk)

*init = function(args)*
Takes in any extra command line arguments and initialises the scripting 
environment

*function request()*
Returns the request to send as HTTP string. Should be a short function so if
need to generate multiple requests then create them all in init(). Called before
every request

*function response(status, headers, body)*
Called after every response this contains the status, headers and body for the 
current response

*function done(summary, latency, requests)*
Called after benchmarking is done, summary is a table containing the result data
and two stats objects with per-request latency and per-thread request rate

### What I want differently

I'd like to be able to collect my own stats from responses and put them into
stats objects.
