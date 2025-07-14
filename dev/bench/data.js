window.BENCHMARK_DATA = {
  "lastUpdate": 1752496938293,
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
          "id": "f10db1c516f3d1885f5917f13ad9dbbed764c0ea",
          "message": "feat: deploy docs",
          "timestamp": "2025-07-14T14:38:56+02:00",
          "tree_id": "ec1c47fb48b9983de70b9181bcbb275bbc629b84",
          "url": "https://github.com/cestef/braise/commit/f10db1c516f3d1885f5917f13ad9dbbed764c0ea"
        },
        "date": 1752496937656,
        "tool": "cargo",
        "benches": [
          {
            "name": "lexer/tokenize/small",
            "value": 791,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "lexer/tokenize/medium",
            "value": 3458,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "lexer/tokenize/large",
            "value": 8446,
            "range": "± 333",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/small",
            "value": 13647,
            "range": "± 320",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/medium",
            "value": 52485,
            "range": "± 147",
            "unit": "ns/iter"
          },
          {
            "name": "parser/parse/large",
            "value": 81279,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/small",
            "value": 17966,
            "range": "± 510",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/medium",
            "value": 61757,
            "range": "± 3470",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/large",
            "value": 94365,
            "range": "± 798",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/small",
            "value": 1519,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/medium",
            "value": 4938,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "runtime/execute/large",
            "value": 7639,
            "range": "± 38",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}