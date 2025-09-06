${
    let code = "";

    // Header column count helper (for padding)
    code += `        private static int HeaderColumnCount(Polygen.Common.CsvIndexHeader h)`+"\n";
    code += `        {`+"\n";
    code += `            if (h.Index >= 0)`+"\n";
    code += `            {`+"\n";
    code += `                return 1;`+"\n";
    code += `            }`+"\n";
    code += `            int n = 0;`+"\n";
    code += `            if (h.IndexList != null)`+"\n";
    code += `            {`+"\n";
    code += `                for (int i = 0; i < h.IndexList.Count; i++)`+"\n";
    code += `                {`+"\n";
    code += `                    n += HeaderColumnCount(h.IndexList[i]);`+"\n";
    code += `                }`+"\n";
    code += `            }`+"\n";
    code += `            return n;`+"\n";
    code += `        }`+"\n";

    // Append/ToRow with provided header tree (non-list 먼저, list 나중 순서로 소비)
    code += `        public static void AppendRowWithHeader(Polygen.Common.CsvIndexHeader h, in ${full_t} obj, List<string> cols, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
    code += `        {`+"\n";
    code += `            int __idx = 0;`+"\n";
    // 1) Non-list fields
    for it in s.items { 
        if it.is_field() {
            let f = it.as_field();
            let tf = f.field_type;
            let base = if tf.is_option { tf.inner } else { tf };
            if !base.is_list {
                let field_name = f.name;
                code += `            {`+"\n";
                code += `                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;`+"\n";
                if base.is_enum {
                    code += `              if (__h != null && __h.Index >= 0)`+"\n";
                    code += `              {`+"\n";
                    code += `                  cols.Add(obj.${field_name}.ToString());`+"\n";
                    code += `              }`+"\n";
                    code += `              else if (__h != null && __h.IndexList != null)`+"\n";
                    code += `              {`+"\n";
                    code += `                  int pad = HeaderColumnCount(__h);`+"\n";
                    code += `                  for (int k = 0; k < pad; k++)`+"\n";
                    code += `                  {`+"\n";
                    code += `                      cols.Add(string.Empty);`+"\n";
                    code += `                  }`+"\n";
                    code += `              }`+"\n";
                    code += `              else`+"\n";
                    code += `              {`+"\n";
                    code += `                  cols.Add(string.Empty);`+"\n";
                    code += `              }`+"\n";
                } else if base.is_primitive || base.lang_type == "string" {
                    code += `              if (__h != null && __h.Index >= 0)`+"\n";
                    code += `              {`+"\n";
                    code += `                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.${field_name}));`+"\n";
                    code += `              }`+"\n";
                    code += `              else`+"\n";
                    code += `              {`+"\n";
                    code += `                  cols.Add(string.Empty);`+"\n";
                    code += `              }`+"\n";
                } else {
                    let fqn = base.fqn; 
                    let call_ns = ""; 
                    let child_name = ""; 
                    let parts = fqn.split("."); 
                    let i = 0; 
                    for seg in parts { 
                        if i < parts.len - 1 { 
                            if call_ns != "" { call_ns += "."; } 
                            call_ns += seg; 
                        } else { child_name = seg; } 
                        i += 1; 
                    }
                    let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                    let es_nl = csv_utils::find_embedded_struct(s, base.type_name);
                    code += `              if (__h != null && __h.IndexList != null)`+"\n";
                    code += `              {`+"\n";
                    code += `                  if (obj.${field_name} != null)`+"\n";
                    code += `                  {`+"\n";
                    if es_nl != () {
                        let owner_fqn_nl = s.fqn;
                        code += `                      global::Csv.${owner_fqn_nl}.${es_nl.name}.AppendRowWithHeader(__h, obj.${field_name}, cols, gap);`+"\n";
                    } else {
                        code += `                      ${call_prefix}${child_name}.AppendRowWithHeader(__h, obj.${field_name}, cols, gap);`+"\n";
                    }
                    code += `                  }`+"\n";
                    code += `                  else`+"\n";
                    code += `                  {`+"\n";
                    code += `                      int pad = HeaderColumnCount(__h); for (int k=0;k<pad;k++) cols.Add(string.Empty);`+"\n";
                    code += `                  }`+"\n";
                    code += `              }`+"\n";
                    code += `              else`+"\n";
                    code += `              {`+"\n";
                    code += `                  cols.Add(string.Empty);`+"\n";
                    code += `              }`+"\n";
                }
                code += `            }`+"\n";
            }
        }
    } // end non-list

    // 2) List fields
    for it in s.items {
        if it.is_field() {
            let f = it.as_field();
            let tf = f.field_type;
            let base = if tf.is_option { tf.inner } else { tf };
            if base.is_list {
                let inner = base;
                code += `            {`+"\n";
                code += `                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;`+"\n";
                if inner.is_enum {
                    code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                    code += `              for (int j=0;j<count;j++)`+"\n";
                    code += `              {`+"\n";
                    code += `                if (obj.${f.name} != null && j < obj.${f.name}.Count)`+"\n";
                    code += `                {`+"\n";
                    code += `                    var v = obj.${f.name}[j];`+"\n";
                    code += `                    cols.Add(v.ToString());`+"\n";
                    code += `                }`+"\n";
                    code += `                else`+"\n";
                    code += `                {`+"\n";
                    code += `                    cols.Add(string.Empty);`+"\n";
                    code += `                }`+"\n";
                    code += `              }`+"\n";
                } else if inner.is_primitive || inner.lang_type == "string" {
                    code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                    code += `              for (int j=0;j<count;j++)`+"\n";
                    code += `              {`+"\n";
                    code += `                if (obj.${f.name} != null && j < obj.${f.name}.Count)`+"\n";
                    code += `                {`+"\n";
                    code += `                    var v = obj.${f.name}[j];`+"\n";
                    code += `                    cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(v));`+"\n";
                    code += `                }`+"\n";
                    code += `                else`+"\n";
                    code += `                {`+"\n";
                    code += `                    cols.Add(string.Empty);`+"\n";
                    code += `                }`+"\n";
                    code += `              }`+"\n";
                } else {
                    // Prefer embedded struct Csv under current owner when present; otherwise use child's own namespace
                    let es = csv_utils::find_embedded_struct(s, inner.type_name);
                    if es != () {
                        code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                        code += `              for (int j=0;j<count;j++)`+"\n";
                        code += `              {`+"\n";
                        code += `                  var subH = __h.IndexList[j]; `+"\n";
                        code += `                  if (obj.${f.name} != null && j < obj.${f.name}.Count && obj.${f.name}[j] != null)`+"\n";
                        code += `                  {`+"\n";
                        code += `                      var sub = obj.${f.name}[j];`+"\n";
                    let owner_fqn = s.fqn;
                    code += `                      global::Csv.${owner_fqn}.${es.name}.AppendRowWithHeader(subH, sub, cols, gap);`+"\n";
                        code += `                  }`+"\n";
                        code += `                  else`+"\n";
                        code += `                  {`+"\n";
                        code += `                      int pad = HeaderColumnCount(subH); for (int k=0;k<pad;k++) cols.Add(string.Empty);`+"\n";
                        code += `                  }`+"\n";
                        code += `              }`+"\n";
                    } else {
                        let fqn = inner.fqn; let call_ns = ""; let child_name = ""; let parts = fqn.split("."); let i = 0; 
                        for seg in parts { if i < parts.len - 1 { if call_ns != "" { call_ns += "."; } call_ns += seg; } else { child_name = seg; } i += 1; }
                        let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                        code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                        code += `              for (int j=0;j<count;j++)`+"\n";
                        code += `              {`+"\n";
                        code += `                  var subH = __h.IndexList[j]; `+"\n";
                        code += `                  if (obj.${f.name} != null && j < obj.${f.name}.Count && obj.${f.name}[j] != null)`+"\n";
                        code += `                  {`+"\n";
                        code += `                      var sub = obj.${f.name}[j];`+"\n";
                        code += `                      ${call_prefix}${child_name}.AppendRowWithHeader(subH, sub, cols, gap);`+"\n";
                        code += `                  }`+"\n";
                        code += `                  else`+"\n";
                        code += `                  {`+"\n";
                        code += `                      int pad = HeaderColumnCount(subH); for (int k=0;k<pad;k++) cols.Add(string.Empty);`+"\n";
                        code += `                  }`+"\n";
                        code += `              }`+"\n";
                    }
                }
                code += `            }`+"\n";
            }
        }
    } // end list
    code += `        }`+"\n";

    code += `        public static string[] ToRowWithHeader(Polygen.Common.CsvIndexHeader h, in ${full_t} obj, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var list = new List<string>(); AppendRowWithHeader(h, obj, list, gap); return list.ToArray(); }`+"\n";

    // Build write-time header tree from items (dynamic list lengths, present nested shapes)
    code += `        public static Polygen.Common.CsvIndexHeader BuildWriteHeaderFromItems(System.Collections.Generic.IEnumerable<${full_t}> items)`+"\n";
    code += `        {`+"\n";
    code += `            var root = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };`+"\n";
    code += `            var __nonList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();`+"\n";
    let has_list_items = false;
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };
        if base.is_list { has_list_items = true; }
    } }
    if has_list_items {
        code += `            var __lists = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();`+"\n";
    }
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let tf = f.field_type;
        let base = if tf.is_option { tf.inner } else { tf };
        if base.is_list {
            let inner = base.inner;
            code += `            {`+"\n";
            code += `                var ch = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };`+"\n";
            // compute max count
            code += `              int __max = 0; `+"\n";
            code += `              foreach (var __it in items)`+"\n";
            code += `              {`+"\n";
            code += `                  var __lst = __it.${f.name};`+"\n";
            code += `                  if (__lst != null && __lst.Count > __max)`+"\n";
            code += `                  {`+"\n";
            code += `                      __max = __lst.Count;`+"\n";
            code += `                  }`+"\n";
            code += `              }`+"\n";
            if inner.is_primitive || inner.lang_type == "string" || inner.is_enum {
                code += `              for (int i=0;i<__max;i++)`+"\n";
                code += `              {`+"\n";
                code += `                  ch.IndexList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });`+"\n";
                code += `              }`+"\n";
            } else {
                let call_ns = inner.namespace_fqn; let child_name = inner.type_name;
                let type_fqn = if call_ns == "" { child_name } else { call_ns + "." + child_name };
                let es = csv_utils::find_embedded_struct(s, inner.type_name);
                code += `              for (int i=0;i<__max;i++)`+"\n";
                code += `              {`+"\n";
                code += `                var coll = new System.Collections.Generic.List<global::${type_fqn}>(); `+"\n";
                code += `                foreach (var __it in items)`+"\n";
                code += `                {`+"\n";
                code += `                    var __lst = __it.${f.name};`+"\n";
                code += `                    if (__lst != null && __lst.Count > i)`+"\n";
                code += `                    {`+"\n";
                code += `                        var cand = __lst[i];`+"\n";
                code += `                        if (cand != null)`+"\n";
                code += `                        {`+"\n";
                code += `                            coll.Add(cand);`+"\n";
                code += `                        }`+"\n";
                code += `                    }`+"\n";
                code += `                } `+"\n";
                if es != () {
                    let owner_fqn_b = s.fqn;
                    code += `                ch.IndexList.Add(coll.Count > 0 ? global::Csv.`+owner_fqn_b+`.`+es.name+`.BuildWriteHeaderFromItems(coll) : new Polygen.Common.CsvIndexHeader());`+"\n";
                } else {
                    let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                    code += `                ch.IndexList.Add(coll.Count > 0 ? ${call_prefix}`+child_name+`.BuildWriteHeaderFromItems(coll) : new Polygen.Common.CsvIndexHeader());`+"\n";
                }
                code += `              }`+"\n";
            }
            code += `              __lists.Add(ch);`+"\n";
            code += `            }`+"\n";
        } else {
            if base.is_primitive || base.lang_type == "string" || base.is_enum {
                code += `            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });`+"\n";
            } else {
                let call_ns = base.namespace_fqn; let child_name = base.type_name;
                let type_fqn = if call_ns == "" { child_name } else { call_ns + "." + child_name };
                let es_nl2 = csv_utils::find_embedded_struct(s, base.type_name);
                code += `            {`+"\n";
                code += `              Polygen.Common.CsvIndexHeader best = null;`+"\n";
                code += `              int bestCols = -1;`+"\n";
                code += `              foreach (var __it in items)`+"\n";
                code += `              {`+"\n";
                code += `                  if (__it.${f.name} != null)`+"\n";
                code += `                  {`+"\n";
                if es_nl2 != () {
                    let owner_fqn_nl2 = s.fqn;
                    code += `                      var _single = new System.Collections.Generic.List<global::${owner_fqn_nl2}.${es_nl2.name}>();`+"\n";
                } else {
                    code += `                      var _single = new System.Collections.Generic.List<global::${type_fqn}>();`+"\n";
                }
                code += `                      _single.Add(__it.${f.name});`+"\n";
                if es_nl2 != () {
                    let owner_fqn_nl2b = s.fqn;
                    code += `                      var cand = global::Csv.${owner_fqn_nl2b}.${es_nl2.name}.BuildWriteHeaderFromItems(_single);`+"\n";
                } else {
                    let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                    code += `                      var cand = ${call_prefix}`+child_name+`.BuildWriteHeaderFromItems(_single);`+"\n";
                }
                code += `                      int cols = HeaderColumnCount(cand);`+"\n";
                code += `                      if (cols > bestCols)`+"\n";
                code += `                      {`+"\n";
                code += `                          bestCols = cols;`+"\n";
                code += `                          best = cand;`+"\n";
                code += `                      }`+"\n";
                code += `                  }`+"\n";
                code += `              }`+"\n";
                code += `              var sub = (bestCols >= 0) ? best : new Polygen.Common.CsvIndexHeader();`+"\n";
                code += `              __nonList.Add(sub);`+"\n";
                code += `            }`+"\n";
            }
        }
    } }
    code += `            foreach (var n in __nonList)`+"\n";
    code += `            {`+"\n";
    code += `                root.IndexList.Add(n);`+"\n";
    code += `            }`+"\n";
    if has_list_items {
        code += `            foreach (var l in __lists)`+"\n";
        code += `            {`+"\n";
        code += `                root.IndexList.Add(l);`+"\n";
        code += `            }`+"\n";
    }
    code += `            return root;`+"\n";
    code += `        }`+"\n";

    // Collect write-time header names from the tree
    code += `        public static void CollectWriteHeaderNames(Polygen.Common.CsvIndexHeader h, string prefix, System.Collections.Generic.List<string> names)`+"\n";
    code += `        {`+"\n";
    code += `            int __idx = 0;`+"\n";
    // Non-list first
    for it in s.items { 
        if it.is_field() {
            let f=it.as_field();
            let tf=f.field_type;
            let base= if tf.is_option { tf.inner } else { tf };
            if !base.is_list {
                code += `            {`+"\n";
                code += `                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;`+"\n";
        if base.is_primitive || base.lang_type == "string" || base.is_enum {
                    let field_name = f.name;
                    code += `              names.Add((prefix==string.Empty? "${field_name}" : prefix + "${field_name}"));`+"\n";
                } else {
                    let call_ns = base.namespace_fqn; let child_name = base.type_name;
                    let field_name = f.name;
                    code += `              if (__h != null)`+"\n";
                    code += `              {`+"\n";
                    let es_nl3 = csv_utils::find_embedded_struct(s, base.type_name);
                    if es_nl3 != () {
                        let owner_fqn_nl3 = s.fqn;
                        code += `                  global::Csv.${owner_fqn_nl3}.${es_nl3.name}.CollectWriteHeaderNames(__h, `+"\n";
                    } else {
                        let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                        code += `                  ${call_prefix}${child_name}.CollectWriteHeaderNames(__h, `+"\n";
                    }
                    code += `                  (prefix==string.Empty? "${field_name}." : prefix + "${field_name}."), names);`+"\n";
                    code += `              }`+"\n";
                }
                code += `            }`+"\n";
            }
        } // NL
    }

    // Lists next
    for it in s.items { 
        if it.is_field() {
            let f=it.as_field();
            let tf=f.field_type;
            let base= if tf.is_option { tf.inner } else { tf };
            if base.is_list {
                let inner=base.inner;
                code += `            {`+"\n";
                code += `                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;`+"\n";
        if inner.is_primitive || inner.lang_type == "string" || inner.is_enum {
                    let field_name = f.name;
                    code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                    code += `              for (int j=0;j<count;j++)`+"\n";
                    code += `              {`+"\n";
                    code += `                  names.Add((prefix==string.Empty? "${field_name}[" + j + "]" : prefix + "${field_name}[" + j + "]"));`+"\n";
                    code += `              }`+"\n";
                } else {
                    let call_ns = inner.namespace_fqn; let child_name = inner.type_name;
                    let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                    code += `              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; `+"\n";
                    code += `              for (int j=0;j<count;j++)`+"\n";
                    code += `              {`+"\n";
                    code += `                  var subH = __h.IndexList[j]; `+"\n";
                    code += `                  ${call_prefix}${child_name}.CollectWriteHeaderNames(subH, `+"\n";
                    code += `                  (prefix==string.Empty? "${f.name}[" + j + "]." : prefix + "${f.name}[" + j + "]."), names);`+"\n";
                    code += `              }`+"\n";
                }
                code += `            }`+"\n";
            }
        }
    } // L
    code += `        }`+"\n";

    // High-level write API: builds header from items, writes header + rows
    code += `        public static void WriteCsv(System.Collections.Generic.IEnumerable<${full_t}> items, string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
    code += `        {`+"\n";
    code += `            var list = new System.Collections.Generic.List<`+full_t+`>(); foreach (var it in items) list.Add(it);`+"\n";
    code += `            var h = BuildWriteHeaderFromItems(list);`+"\n";
    code += `            var names = new System.Collections.Generic.List<string>(); CollectWriteHeaderNames(h, string.Empty, names);`+"\n";
    code += `            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));`+"\n";
    code += `            sw.WriteLine(Polygen.Common.CsvUtils.Join(names, sep));`+"\n";
    code += `            foreach (var it in list)`+"\n";
    code += `            {`+"\n";
    code += `                var row = ToRowWithHeader(h, it, gap);`+"\n";
    code += `                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));`+"\n";
    code += `            }`+"\n";
    code += `        }`+"\n";

    code += `        public static void WriteCsvWithHeader(IEnumerable<${full_t}> items, string path, string[] header, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
    code += `        {`+"\n";
    code += `            var h = BuildHeader(header, string.Empty);`+"\n";
    code += `            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));`+"\n";
    code += `            sw.WriteLine(Polygen.Common.CsvUtils.Join(header, sep));`+"\n";
    code += `            foreach (var it in items)`+"\n";
    code += `            {`+"\n";
    code += `                var row = ToRowWithHeader(h, it, gap);`+"\n";
    code += `                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));`+"\n";
    code += `            }`+"\n";
    code += `        }`+"\n";

    // BuildHeader
    code += `        public static Polygen.Common.CsvIndexHeader BuildHeader(string[] header, string prefix)`+"\n";
    code += `        {`+"\n";
    code += `            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);`+"\n";
    code += `            var root = new Polygen.Common.CsvIndexHeader();`+"\n";
    code += `            root.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();`+"\n";
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let tf = f.field_type;
        let base = if tf.is_option { tf.inner } else { tf };
        if base.is_list {
            let inner = base.inner;
            code += `            {`+"\n";
            code += `                var ch = new Polygen.Common.CsvIndexHeader();`+"\n";
            code += `                ch.Index = -1;`+"\n";
            code += `                ch.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();`+"\n";
            if inner.is_primitive || inner.lang_type == "string" || inner.is_enum {
                code += `                for (int i=0;;i++)`+"\n";
                code += `                {`+"\n";
                code += `                    int __ix;`+"\n";
                code += `                    if (!map.TryGetValue(prefix + "${f.name}[" + i + "]", out __ix))`+"\n";
                code += `                    {`+"\n";
                code += `                        break;`+"\n";
                code += `                    }`+"\n";
                code += `                    var leaf = new Polygen.Common.CsvIndexHeader();`+"\n";
                code += `                    leaf.Index = __ix;`+"\n";
                code += `                    ch.IndexList.Add(leaf);`+"\n";
                code += `                }`+"\n";
            } else {
                let es = csv_utils::find_embedded_struct(s, inner.type_name);
                if es != () {
                    code += `                for (int i=0;;i++)`+"\n";
                    code += `                {`+"\n";
                    let owner_fqn = s.fqn;
                    code += `                    var sub = global::Csv.${owner_fqn}.${es.name}.BuildHeader(header, prefix + "${f.name}["+i+"]."); `+"\n";
                    code += `if (!sub.HasAny()) break; ch.IndexList.Add(sub); }`+"\n";
                } else {
                    let call_ns = if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn };
                    code += `                for (int i=0;;i++)`+"\n";
                    code += `                {`+"\n";
                    code += `                    var sub = global::Csv.${call_ns}.${inner.type_name}.BuildHeader(header, prefix + "${f.name}["+i+"]."); `+"\n";
                    code += `if (!sub.HasAny()) break; ch.IndexList.Add(sub); }`+"\n";
                }
            }
            code += `                root.IndexList.Add(ch); }`+"\n";
        } else {
            let es = csv_utils::find_embedded_struct(s, base.type_name);
            if es != () {
                let owner_fqn = s.fqn;
                code += `            root.IndexList.Add(global::Csv.${owner_fqn}.${es.name}.BuildHeader(header, prefix + "${f.name}."));`+"\n";
            } else if base.is_primitive || base.lang_type == "string" || base.is_enum {
                code += `            {`+"\n";
                code += `                int __ix;`+"\n";
                code += `                if (map.TryGetValue(prefix + "${f.name}", out __ix))`+"\n";
                code += `                {`+"\n";
                code += `                    var leaf = new Polygen.Common.CsvIndexHeader();`+"\n";
                code += `                    leaf.Index = __ix;`+"\n";
                code += `                    root.IndexList.Add(leaf);`+"\n";
                code += `                }`+"\n";
                code += `            }`+"\n";
            } else {
                let call_ns = if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn };
                code += `            root.IndexList.Add(global::Csv.${call_ns}.${base.type_name}.BuildHeader(header, prefix + "${f.name}."));`+"\n";
            }
            //code += `            }`+"\n";
        }
    } }
    code += `            return root;`+"\n";
    code += `        }`+"\n";

    code
}
