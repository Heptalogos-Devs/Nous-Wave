# Episode Runtime Integration

状态：IMPLEMENTATION-AUTHORIZING

The public Memory service exposes typed Episode create/read/history/list/revise/link/lifecycle operations. Official Client bindings are generated from canonical Proto.

WorkContext may retain only an exact `EpisodeRevision` ref. Runtime validates and surfaces that ref; it does not copy Episode content or add a second ranked Episode lane.

Final materialization may expose an exact current EpisodeRevision candidate using its title/boundary explanation and exact provenance. General lexical, dense, and topology Episode ranking is deferred.

Qualification must prove ordered members, revision history, same-track overlap rejection, different-track overlap acceptance, hierarchy cycle rejection, provenance trace, restart recovery, WorkContext continuation, lifecycle behavior, and purge redaction.
