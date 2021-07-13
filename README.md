# murk

Murk aims to learn lessons from wrk and wrk2 as well as fulfilling some of my
own needs. But mainly it's also for my own enjoyment and not for serious use
by anybody.

Part of the reason for doing this is I work on services that take larger bodies
with longer response times where we care about things like Real Time Factor. So
while testing things want to see how things like load relate to RTF, and
generate fancy graphs. So I'm building in a python scripting engine that aims
to be more flexible. Also, using HdrHistogram and maybe providing the ability to
register extra histograms.
