import json

print(json.dumps({
    "schema": "project.ops.status/v1",
    "project": "cogwheel-fixture",
    "generated_at": "2026-08-25T12:00:00Z",
    "manifest": {
        "schema": "project.concerns/v1",
        "path": ".ops/concerns.toml",
    },
    "producer": {
        "id": "cogwheel-fixture.status",
        "session_id": "cogwheel-primary-1",
    },
    "authority": {"kind": "producer-local"},
    "concerns": [{
        "id": "cogwheel.queue.high",
        "question": "cogwheel.question.queue-high/v1",
        "profile": "cogwheel.profile.queue-high-18/v1",
        "required": True,
        "description": "Is the bounded queue depth at least 18?",
        "observation": {
            "observation_present": True,
            "local_state": "COGWHEEL_CHATTERING",
            "domain_state": "TEETH_ENGAGED",
            "observed_at": "2026-08-25T12:00:00Z",
            "valid_for_seconds": 600,
            "reason": "bounded unfamiliar-project fixture testimony",
            "facts": {"queue": {"depth": 21}},
        },
    }],
}))
