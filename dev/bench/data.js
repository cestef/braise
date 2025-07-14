window.BENCHMARK_DATA = {
  "lastUpdate": 1752486597758,
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
          "id": "dce797093deb642924f25ff87303b6fc115053a9",
          "message": "fix: push to gh pages",
          "timestamp": "2025-07-14T11:45:56+02:00",
          "tree_id": "13d182b92aeebbc3891ed0c8974cb7c26c24bb43",
          "url": "https://github.com/cestef/braise/commit/dce797093deb642924f25ff87303b6fc115053a9"
        },
        "date": 1752486597520,
        "tool": "cargo",
        "benches": [
          {
            "name": "tokenize/tokenize/simple",
            "value": 1383,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "tokenize/tokenize/web",
            "value": 3436,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "tokenize/tokenize/rust",
            "value": 8047,
            "range": "± 264",
            "unit": "ns/iter"
          },
          {
            "name": "tokenize/tokenize/complex",
            "value": 12573,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "parse/parse/simple",
            "value": 16685,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "parse/parse/web",
            "value": 31519,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "parse/parse/rust",
            "value": 63302,
            "range": "± 288",
            "unit": "ns/iter"
          },
          {
            "name": "parse/parse/complex",
            "value": 162022,
            "range": "± 1224",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/simple",
            "value": 20381,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/web",
            "value": 35583,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/rust",
            "value": 73988,
            "range": "± 1687",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline/full/complex",
            "value": 181488,
            "range": "± 818",
            "unit": "ns/iter"
          },
          {
            "name": "scale/recipes/50",
            "value": 217586,
            "range": "± 4260",
            "unit": "ns/iter"
          },
          {
            "name": "scale/recipes/100",
            "value": 428974,
            "range": "± 1693",
            "unit": "ns/iter"
          },
          {
            "name": "scale/recipes/200",
            "value": 846271,
            "range": "± 2689",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}