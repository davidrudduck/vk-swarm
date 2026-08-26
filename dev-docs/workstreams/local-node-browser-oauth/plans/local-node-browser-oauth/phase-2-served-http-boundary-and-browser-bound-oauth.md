# Phase 2: Served HTTP boundary and browser-bound OAuth

Make the served-router harness cookie-aware, split the node router into explicit public and protected subtrees with an API-terminating fallback and a minimal public auth-state endpoint, and turn the existing Hive handoff into a browser-bound flow that identifies the candidate and pins the owner BEFORE saving daemon credentials or minting a session.
