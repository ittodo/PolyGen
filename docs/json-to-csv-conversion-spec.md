# JSON → Table → CSV Conversion Spec (PolyGen)

목표: PolyGen에서 JSON 파서를 제거하고, 각 언어별로 JSON을 Table(헤더+행)로 변환한 뒤 CSV 매처에 전달하는 일관된 변환 절차를 정의한다. 본 문서는 언어 불문 의사 코드로 스펙을 제시한다.

## 파이프라인 개요
- 입력(JSON 생성기): PolyGen 내부 도메인 모델 → JSON(객체/배열)
- 변환기(언어별 구현):
  - 헤더 생성: `BUILD_HEADER_FROM_JSON(json, strategy, K)`
  - 행 생성: 각 객체를 `FLATTEN_TO_ROW(obj, header)`로 변환
  - CSV 직렬화: `TO_CSV(header, rows, { sep, bom, newline })`
- 매칭기: CSV 매처 요구사항(헤더 포함 여부, 구분자, 개행)에 맞춰 옵션만 조정

## Public 의사 코드 API
- `JSON_TO_TABLE(json, cfg) -> { header: string[], rows: string[][] }`
- `JSON_TO_CSV(json, cfg) -> string`

### cfg (설정)
- `listStrategy`: `'dynamic' | 'fixed'`  // 동적/고정 리스트 전략
- `fixedListMax`: `number`               // `fixed`일 때 K
- `sep`: `string = ','`                  // CSV 구분자
- `newline`: `'\n' | '\r\n' = '\n'`    // 개행 문자
- `bom`: `boolean = false`               // UTF-8 BOM 여부
- `includeHeader`: `boolean = true`      // 헤더 포함 여부

## JSON_TO_TABLE
입력: `json: any`, `cfg`

절차:
```
function JSON_TO_TABLE(json, cfg): { header, rows } {
  { header } = BUILD_HEADER_FROM_JSON(json, cfg.listStrategy, cfg.fixedListMax)
  arr = isArray(json) ? json : [json]
  rows = arr.map(obj => FLATTEN_TO_ROW(obj, header))
  return { header, rows }
}
```

## JSON_TO_CSV
입력: `json: any`, `cfg`

절차:
```
function JSON_TO_CSV(json, cfg): string {
  { header, rows } = JSON_TO_TABLE(json, cfg)
  if (cfg.includeHeader == false) {
    sep = cfg.sep ?? ','
    nl  = cfg.newline ?? '\n'
    return rows.map(r => r.join(sep)).join(nl)
  }
  return TO_CSV(header, rows, { sep: cfg.sep, bom: cfg.bom, newline: cfg.newline })
}
```

## BUILD_HEADER_FROM_JSON
입력: `json`, `strategy`, `K`

절차:
```
function BUILD_HEADER_FROM_JSON(json, strategy, K): { header } {
  { prototypeHeader, observedMaxes } = SCAN_SCHEMA(json)
  if (strategy == 'fixed') {
    maxes = {}
    for col in prototypeHeader {
      if (col contains '[0]') {
        root = rootOf(col)             // e.g., 'items' from 'items[0].id'
        maxes[root] = max(maxes[root] ?? 0, K)
      }
    }
    return { header: BUILD_HEADER(prototypeHeader, maxes) }
  }
  return { header: BUILD_HEADER(prototypeHeader, observedMaxes) }
}
```

## SCAN_SCHEMA (개략)
입력: `json`

절차:
```
function SCAN_SCHEMA(json): { prototypeHeader, observedMaxes } {
  prototypeHeader = []
  observedMaxes = map()  // key: list root, value: max index observed

  function walk(value, path) {
    if (isObject(value)) {
      for (key in value) { walk(value[key], join(path, key)) }
    } else if (isArray(value)) {
      // record max index per list root; prototype uses [0]
      root = path
      for (i = 0; i < value.length; i++) {
        observedMaxes[root] = max(observedMaxes[root] ?? -1, i)
        walk(value[i], path + '[' + i + ']')
      }
      // add prototype path once with [0]
      // e.g., items[0].id, items[0].name ... collected during descent
    } else {
      // primitive leaf → push path into prototype (with any list indices normalized to [0])
      proto = normalizeIndicesToZero(path)  // convert [n] → [0] only at first index per list segment
      prototypeHeader.push(proto)
    }
  }

  walk(json, '')
  prototypeHeader = uniqueStable(prototypeHeader)
  return { prototypeHeader, observedMaxes }
}
```

## BUILD_HEADER
입력: `prototypeHeader`, `listMaxes`

절차:
```
function BUILD_HEADER(prototypeHeader, listMaxes): string[] {
  out = []
  for (col in prototypeHeader) {
    if (containsIndexZero(col)) {
      root = rootOf(col)
      maxI = listMaxes[root] ?? -1
      for (i = 0; i <= maxI; i++) {
        out.push(replaceFirstIndex(col, i))
      }
    } else {
      out.push(col)
    }
  }
  return out
}
```

## FLATTEN_TO_ROW
입력: `obj`, `header`

절차:
```
function FLATTEN_TO_ROW(obj, header): string[] {
  row = []
  for (col in header) {
    tokens = PARSE_PATH(col)  // a.b[0].c → [{key:'a'}, {key:'b', index:0}, {key:'c'}]
    v = GET_AT(obj, tokens)
    row.push(TO_STRING(v))
  }
  return row
}
```

## PARSE_PATH
```
function PARSE_PATH(col): Token[] {
  // Token = { key: string, index?: number }
  // parse segments, handling dot notation and [index]
}
```

## GET_AT
```
function GET_AT(obj, tokens) {
  cur = obj
  for (t in tokens) {
    if (cur == null) return undefined
    if (t.key exists) cur = cur[t.key]
    if (t.index exists) {
      if (!isArray(cur)) return undefined
      cur = cur[t.index]
    }
  }
  return cur
}
```

## TO_STRING
```
function TO_STRING(v): string {
  if (v is null or undefined) return ''
  if (typeof v == 'string') return v
  if (typeof v == 'boolean') return v ? 'true' : 'false'
  if (typeof v == 'number') return isFinite(v) ? String(v) : ''
  return JSON.stringify(v)
}
```

## TO_CSV
입력: `header`, `rows`, `{ sep, bom, newline }`

절차:
```
function TO_CSV(header, rows, opts): string {
  sep = opts.sep ?? ','
  nl  = opts.newline ?? '\n'

  function ESC(s): string {
    if (s contains sep or '"' or '\n' or '\r') {
      return '"' + s.replaceAll('"', '""') + '"'
    }
    return s
  }

  head = header.map(h => ESC(h)).join(sep)
  body = rows.map(r => r.map(c => ESC(c ?? '')).join(sep)).join(nl)
  csv  = head + (rows.length ? nl + body : '')
  return (opts.bom ? '\uFEFF' : '') + csv
}
```

## 구현 가이드
- 스키마가 고정되어야 하면 `listStrategy='fixed'`와 `fixedListMax=K` 사용(헤더 안정성 보장)
- 매처가 헤더를 원하지 않으면 `includeHeader=false`
- 값은 문자열로 직렬화됨(숫자/불린 포함). 타입 보존이 필요하면 언어별 포맷터/파서를 추가
- 중첩 배열은 첫 인덱스만 확장(필요 시 더 깊은 인덱싱 정책을 언어별 정책으로 확장 가능)
