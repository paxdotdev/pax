# Paxgen

LLM prompt generator to create Pax UIs.

This works by embedding parts of the Pax documentation into the
prompt as one concatenated string, as well as examples in order
to perform in-context learning.

The way the parts of the documentation and examples are identified
is by substring matching; this is obviously very brittle and would
scale better if those were refactored to be LLM-native. It could be
the case that we could just dump all docs and examples into context
and it would work just as well.

This last seemed to work well at the following commit on `pax-docs`:

```
commit c1cec6a3fa3b665d30cb9afafac7ac019aca84d9
Author: Warfa Jibril <warfaj@gmail.com>
Date:   Wed May 22 17:53:57 2024 -0700

    added pax-cli to critical path
```