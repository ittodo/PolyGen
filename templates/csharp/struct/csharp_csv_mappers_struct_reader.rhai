${
    let code = "";

    // FromRowWithHeader
    code += `        public static ${full_t} FromRowWithHeader(Polygen.Common.CsvIndexHeader h, string[] row, Polygen.Common.CsvUtils.GapMode gap)`+"\n";
    code += `        {`+"\n";
    code += `            var obj = new ${full_t}();`+"\n";
    code += `            int __i = 0;`+"\n";
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };
        code += `            {`+"\n";
        code += `                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;`+"\n";
        if base.is_list {
            let inner = base.inner;
            if inner.is_primitive || inner.lang_type == "string" || inner.is_enum {
                let cs_inner = type_utils::map_type(inner);
                let generic_t = if inner.is_enum { "global::" + inner.fqn } else { cs_inner };
                code += `                {`+"\n";
                code += `                    var list = new System.Collections.Generic.List<${cs_inner}>();`+"\n";
                code += `                    if (__h != null && __h.IndexList != null)`+"\n";
                code += `                    {`+"\n";
                code += `                        for (int i = 0; i < __h.IndexList.Count; i++)`+"\n";
                code += `                        {`+"\n";
                code += `                            var subH = __h.IndexList[i];`+"\n";
                code += `                            if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row))`+"\n";
                code += `                            {`+"\n";
                code += `                                if (i == 0 || gap == Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
                code += `                                {`+"\n";
                code += `                                    break;`+"\n";
                code += `                                }`+"\n";
                code += `                                else`+"\n";
                code += `                                {`+"\n";
                code += `                                    continue;`+"\n";
                code += `                                }`+"\n";
                code += `                            }`+"\n";
                code += `                            var v = DataSourceFactory.ConvertValue<${generic_t}>(row[subH.Index]);`+"\n";
                code += `                            list.Add(v);`+"\n";
                code += `                        }`+"\n";
                code += `                    }`+"\n";
                code += `                    obj.${f.name} = list;`+"\n";
                code += `                }`+"\n";
            } else {
                let call_ns = inner.namespace_fqn; let child_name = inner.type_name;
                let es = csv_utils::find_embedded_struct(s, inner.type_name);
                code += `                {`+"\n";
                code += `                    var list = new System.Collections.Generic.List<global::${inner.fqn}>();`+"\n";
                code += `                    if (__h != null && __h.IndexList != null)`+"\n";
                code += `                    {`+"\n";
                code += `                        for (int i = 0; i < __h.IndexList.Count; i++)`+"\n";
                code += `                        {`+"\n";
                code += `                            var subH = __h.IndexList[i];`+"\n";
                code += `                            if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row))`+"\n";
                code += `                            {`+"\n";
                code += `                                if (i == 0 || gap == Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
                code += `                                {`+"\n";
                code += `                                    break;`+"\n";
                code += `                                }`+"\n";
                code += `                                else`+"\n";
                code += `                                {`+"\n";
                code += `                                    continue;`+"\n";
                code += `                                }`+"\n";
                code += `                            }`+"\n";
                if es != () {
                    let owner_fqn = s.fqn;
                    code += `                            var sub = global::Csv.`+owner_fqn+`.`+es.name+`.FromRowWithHeader(subH, row, gap);`+"\n";
                } else {
                    let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                    code += `                            var sub = ${call_prefix}${child_name}.FromRowWithHeader(subH, row, gap);`+"\n";
                }
                code += `                            list.Add(sub);`+"\n";
                code += `                        }`+"\n";
                code += `                    }`+"\n";
                code += `                    obj.${f.name} = list;`+"\n";
                code += `                }`+"\n";
            }
        } else {
            let es = csv_utils::find_embedded_struct(s, base.type_name);
            if es != () {
                code += `            if (!Polygen.Common.CsvUtils.HeaderHasValues(__h, row))`+"\n";
                code += `            {`+"\n";
                code += `                obj.${f.name} = null;`+"\n";
                code += `            }`+"\n";
                code += `            else`+"\n";
                code += `            {`+"\n";
                let owner_fqn = s.fqn;
                code += `                obj.${f.name} = global::Csv.${owner_fqn}.${es.name}.FromRowWithHeader(__h, row, gap);`+"\n";
                code += `            }`+"\n";
            } else if base.is_primitive || base.lang_type == "string" || base.is_enum {
                let cs_t = type_utils::map_type(base);
                let generic_t = if base.is_enum { "global::" + base.fqn } else { cs_t };
                code += `            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)`+"\n";
                code += `            {`+"\n";
                code += `                var __cell = row[__h.Index];`+"\n";
                code += `                obj.${f.name} = DataSourceFactory.ConvertValue<${generic_t}>(__cell);`+"\n";
                code += `            }`+"\n";
            } else {
                let call_ns = base.namespace_fqn; let child_name = base.type_name;
                let call_prefix = if call_ns == "" { "global::Csv." } else { "global::Csv." + call_ns + "." };
                code += `            if (!Polygen.Common.CsvUtils.HeaderHasValues(__h, row))`+"\n";
                code += `            {`+"\n";
                code += `                obj.${f.name} = null;`+"\n";
                code += `            }`+"\n";
                code += `            else`+"\n";
                code += `            {`+"\n";
                code += `                obj.${f.name} = ${call_prefix}${child_name}.FromRowWithHeader(__h, row, gap);`+"\n";
                code += `            }`+"\n";
            }
        }
        code += `            }`+"\n";
    } }
    code += `            return obj;`+"\n";
    code += `        }`+"\n";

    // ReadCsvFast
    code += `        public static System.Collections.Generic.IEnumerable<${full_t}> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
    code += `        {`+"\n";
    code += `            var lines = File.ReadAllLines(path);`+"\n";
    code += `            if (lines.Length == 0)`+"\n";
    code += `            {`+"\n";
    code += `                yield break;`+"\n";
    code += `            }`+"\n";
    code += `            var header = lines[0].Split(sep);`+"\n";
    code += `            var h = BuildHeader(header, string.Empty);`+"\n";
    code += `            for (int i = 1; i < lines.Length; i++)`+"\n";
    code += `            {`+"\n";
    code += `                var row = lines[i].Split(sep);`+"\n";
    code += `                if (!Polygen.Common.CsvUtils.HeaderHasValues(h, row))`+"\n";
    code += `                {`+"\n";
    code += `                    if (gap == Polygen.Common.CsvUtils.GapMode.Break)`+"\n";
    code += `                    {`+"\n";
    code += `                        break;`+"\n";
    code += `                    }`+"\n";
    code += `                    else`+"\n";
    code += `                    {`+"\n";
    code += `                        continue;`+"\n";
    code += `                    }`+"\n";
    code += `                }`+"\n";
    code += `                yield return FromRowWithHeader(h, row, gap);`+"\n";
    code += `            }`+"\n";
    code += `        }`+"\n";

    code
}
