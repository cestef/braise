window.BENCHMARK_DATA = {
  "lastUpdate": 1752493415092,
  "repoUrl": "https://github.com/cestef/braise",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "root@cstef.dev",
            "name": "cstef",
            "username": "cestef"
          },
          "committer": {
            "email": "root@cstef.dev",
            "name": "cstef",
            "username": "cestef"
          },
          "distinct": true,
          "id": "baa5d28d86df36460b2bd37fa97505262cff168e",
          "message": "fix: remove useless lib.rs",
          "timestamp": "2025-07-14T13:39:56+02:00",
          "tree_id": "7c0be87d5644509b970c703514ca631c13ba73ac",
          "url": "https://github.com/cestef/braise/commit/baa5d28d86df36460b2bd37fa97505262cff168e"
        },
        "date": 1752493414840,
        "tool": "cargo",
        "benches": [
          {
            "name": "lexer/tokenize/small",
            "value": 784,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lexer/tokenize/medium",
            "value": 3483,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "lexer/tokenize/large",
            "value": 8587,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/small",
            "value": 13391,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/medium",
            "value": 52769,
            "range": "± 529",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/large",
            "value": 79021,
            "range": "± 1240",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/small",
            "value": 16122,
            "range": "± 370",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/medium",
            "value": 60839,
            "range": "± 948",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/large",
            "value": 92716,
            "range": "± 590",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/small",
            "value": 1512,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/medium",
            "value": 5077,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/large",
            "value": 8297,
            "range": "± 17",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
};
