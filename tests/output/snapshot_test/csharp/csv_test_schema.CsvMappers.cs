using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.Globalization;
using System.IO;
using System.Text;
using Polygen.Common;
namespace Csv.test.csv
{
	public static class Point
	{
	        private static int HeaderColumnCount(Polygen.Common.CsvIndexHeader h)
	        {
	            if (h.Index >= 0)
	            {
	                return 1;
	            }
	            int n = 0;
	            if (h.IndexList != null)
	            {
	                for (int i = 0; i < h.IndexList.Count; i++)
	                {
	                    n += HeaderColumnCount(h.IndexList[i]);
	                }
	            }
	            return n;
	        }
	        public static void AppendRowWithHeader(Polygen.Common.CsvIndexHeader h, in global::test.csv.Point obj, List<string> cols, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            int __idx = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.x));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.y));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	        }
	        public static string[] ToRowWithHeader(Polygen.Common.CsvIndexHeader h, in global::test.csv.Point obj, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var list = new List<string>(); AppendRowWithHeader(h, obj, list, gap); return list.ToArray(); }
	        public static Polygen.Common.CsvIndexHeader BuildWriteHeaderFromItems(System.Collections.Generic.IEnumerable<global::test.csv.Point> items)
	        {
	            var root = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };
	            var __nonList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            foreach (var n in __nonList)
	            {
	                root.IndexList.Add(n);
	            }
	            return root;
	        }
	        public static void CollectWriteHeaderNames(Polygen.Common.CsvIndexHeader h, string prefix, System.Collections.Generic.List<string> names)
	        {
	            int __idx = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "x" : prefix + "x"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "y" : prefix + "y"));
	            }
	        }
	        public static void WriteCsv(System.Collections.Generic.IEnumerable<global::test.csv.Point> items, string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var list = new System.Collections.Generic.List<global::test.csv.Point>(); foreach (var it in items) list.Add(it);
	            var h = BuildWriteHeaderFromItems(list);
	            var names = new System.Collections.Generic.List<string>(); CollectWriteHeaderNames(h, string.Empty, names);
	            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));
	            sw.WriteLine(Polygen.Common.CsvUtils.Join(names, sep));
	            foreach (var it in list)
	            {
	                var row = ToRowWithHeader(h, it, gap);
	                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));
	            }
	        }
	        public static void WriteCsvWithHeader(IEnumerable<global::test.csv.Point> items, string path, string[] header, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var h = BuildHeader(header, string.Empty);
	            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));
	            sw.WriteLine(Polygen.Common.CsvUtils.Join(header, sep));
	            foreach (var it in items)
	            {
	                var row = ToRowWithHeader(h, it, gap);
	                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));
	            }
	        }
	        public static Polygen.Common.CsvIndexHeader BuildHeader(string[] header, string prefix)
	        {
	            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
	            var root = new Polygen.Common.CsvIndexHeader();
	            root.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "x", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "y", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            return root;
	        }

	        public static global::test.csv.Point FromRowWithHeader(Polygen.Common.CsvIndexHeader h, string[] row, Polygen.Common.CsvUtils.GapMode gap)
	        {
	            var obj = new global::test.csv.Point();
	            int __i = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.x = DataSourceFactory.ConvertValue<int>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.y = DataSourceFactory.ConvertValue<int>(__cell);
	            }
	            }
	            return obj;
	        }
	        public static System.Collections.Generic.IEnumerable<global::test.csv.Point> ReadRowsWithHeader(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var lines = File.ReadAllLines(path);
	            if (lines.Length == 0)
	            {
	                yield break;
	            }
	            var header = lines[0].Split(sep);
	            var h = BuildHeader(header, string.Empty);
	            for (int i = 1; i < lines.Length; i++)
	            {
	                var row = lines[i].Split(sep);
	                if (!Polygen.Common.CsvUtils.HeaderHasValues(h, row))
	                {
	                    if (gap == Polygen.Common.CsvUtils.GapMode.Break)
	                    {
	                        break;
	                    }
	                    else
	                    {
	                        continue;
	                    }
	                }
	                yield return FromRowWithHeader(h, row, gap);
	            }
	        }
	        public static System.Collections.Generic.IEnumerable<global::test.csv.Point> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	            => ReadRowsWithHeader(path, sep, gap);

	}


	public static class TestObject
	{
	        private static int HeaderColumnCount(Polygen.Common.CsvIndexHeader h)
	        {
	            if (h.Index >= 0)
	            {
	                return 1;
	            }
	            int n = 0;
	            if (h.IndexList != null)
	            {
	                for (int i = 0; i < h.IndexList.Count; i++)
	                {
	                    n += HeaderColumnCount(h.IndexList[i]);
	                }
	            }
	            return n;
	        }
	        public static void AppendRowWithHeader(Polygen.Common.CsvIndexHeader h, in global::test.csv.TestObject obj, List<string> cols, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            int __idx = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.id));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.name));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.active));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(obj.score));
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.Index >= 0)
	              {
	                  cols.Add(obj.color.ToString());
	              }
	              else if (__h != null && __h.IndexList != null)
	              {
	                  int pad = HeaderColumnCount(__h);
	                  for (int k = 0; k < pad; k++)
	                  {
	                      cols.Add(string.Empty);
	                  }
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null && __h.IndexList != null)
	              {
	                  if (obj.location != null)
	                  {
	                      global::Csv.test.csv.Point.AppendRowWithHeader(__h, obj.location, cols, gap);
	                  }
	                  else
	                  {
	                      int pad = HeaderColumnCount(__h); for (int k=0;k<pad;k++) cols.Add(string.Empty);
	                  }
	              }
	              else
	              {
	                  cols.Add(string.Empty);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; 
	              for (int j=0;j<count;j++)
	              {
	                if (obj.tags != null && j < obj.tags.Count)
	                {
	                    var v = obj.tags[j];
	                    cols.Add(Polygen.Common.CsvUtils.ToStringInvariant(v));
	                }
	                else
	                {
	                    cols.Add(string.Empty);
	                }
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; 
	              for (int j=0;j<count;j++)
	              {
	                  var subH = __h.IndexList[j]; 
	                  if (obj.history != null && j < obj.history.Count && obj.history[j] != null)
	                  {
	                      var sub = obj.history[j];
	                      global::Csv.test.csv.Point.AppendRowWithHeader(subH, sub, cols, gap);
	                  }
	                  else
	                  {
	                      int pad = HeaderColumnCount(subH); for (int k=0;k<pad;k++) cols.Add(string.Empty);
	                  }
	              }
	            }
	        }
	        public static string[] ToRowWithHeader(Polygen.Common.CsvIndexHeader h, in global::test.csv.TestObject obj, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var list = new List<string>(); AppendRowWithHeader(h, obj, list, gap); return list.ToArray(); }
	        public static Polygen.Common.CsvIndexHeader BuildWriteHeaderFromItems(System.Collections.Generic.IEnumerable<global::test.csv.TestObject> items)
	        {
	            var root = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };
	            var __nonList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	            var __lists = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            {
	                var ch = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };
	              int __max = 0; 
	              foreach (var __it in items)
	              {
	                  var __lst = __it.tags;
	                  if (__lst != null && __lst.Count > __max)
	                  {
	                      __max = __lst.Count;
	                  }
	              }
	              for (int i=0;i<__max;i++)
	              {
	                  ch.IndexList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	              }
	              __lists.Add(ch);
	            }
	            __nonList.Add(new Polygen.Common.CsvIndexHeader { Index = 0 });
	            {
	              Polygen.Common.CsvIndexHeader best = null;
	              int bestCols = -1;
	              foreach (var __it in items)
	              {
	                  if (__it.location != null)
	                  {
	                      var _single = new System.Collections.Generic.List<global::test.csv.Point>();
	                      _single.Add(__it.location);
	                      var cand = global::Csv.test.csv.Point.BuildWriteHeaderFromItems(_single);
	                      int cols = HeaderColumnCount(cand);
	                      if (cols > bestCols)
	                      {
	                          bestCols = cols;
	                          best = cand;
	                      }
	                  }
	              }
	              var sub = (bestCols >= 0) ? best : new Polygen.Common.CsvIndexHeader();
	              __nonList.Add(sub);
	            }
	            {
	                var ch = new Polygen.Common.CsvIndexHeader { Index = -1, IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>() };
	              int __max = 0; 
	              foreach (var __it in items)
	              {
	                  var __lst = __it.history;
	                  if (__lst != null && __lst.Count > __max)
	                  {
	                      __max = __lst.Count;
	                  }
	              }
	              for (int i=0;i<__max;i++)
	              {
	                var coll = new System.Collections.Generic.List<global::test.csv.Point>(); 
	                foreach (var __it in items)
	                {
	                    var __lst = __it.history;
	                    if (__lst != null && __lst.Count > i)
	                    {
	                        var cand = __lst[i];
	                        if (cand != null)
	                        {
	                            coll.Add(cand);
	                        }
	                    }
	                } 
	                ch.IndexList.Add(coll.Count > 0 ? global::Csv.test.csv.Point.BuildWriteHeaderFromItems(coll) : new Polygen.Common.CsvIndexHeader());
	              }
	              __lists.Add(ch);
	            }
	            foreach (var n in __nonList)
	            {
	                root.IndexList.Add(n);
	            }
	            foreach (var l in __lists)
	            {
	                root.IndexList.Add(l);
	            }
	            return root;
	        }
	        public static void CollectWriteHeaderNames(Polygen.Common.CsvIndexHeader h, string prefix, System.Collections.Generic.List<string> names)
	        {
	            int __idx = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "id" : prefix + "id"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "name" : prefix + "name"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "active" : prefix + "active"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "score" : prefix + "score"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              names.Add((prefix==string.Empty? "color" : prefix + "color"));
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              if (__h != null)
	              {
	                  global::Csv.test.csv.Point.CollectWriteHeaderNames(__h, 
	                  (prefix==string.Empty? "location." : prefix + "location."), names);
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; 
	              for (int j=0;j<count;j++)
	              {
	                  names.Add((prefix==string.Empty? "tags[" + j + "]" : prefix + "tags[" + j + "]"));
	              }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __idx < h.IndexList.Count) ? h.IndexList[__idx++] : null;
	              int count = (__h != null && __h.IndexList != null) ? __h.IndexList.Count : 0; 
	              for (int j=0;j<count;j++)
	              {
	                  var subH = __h.IndexList[j]; 
	                  global::Csv.test.csv.Point.CollectWriteHeaderNames(subH, 
	                  (prefix==string.Empty? "history[" + j + "]." : prefix + "history[" + j + "]."), names);
	              }
	            }
	        }
	        public static void WriteCsv(System.Collections.Generic.IEnumerable<global::test.csv.TestObject> items, string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var list = new System.Collections.Generic.List<global::test.csv.TestObject>(); foreach (var it in items) list.Add(it);
	            var h = BuildWriteHeaderFromItems(list);
	            var names = new System.Collections.Generic.List<string>(); CollectWriteHeaderNames(h, string.Empty, names);
	            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));
	            sw.WriteLine(Polygen.Common.CsvUtils.Join(names, sep));
	            foreach (var it in list)
	            {
	                var row = ToRowWithHeader(h, it, gap);
	                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));
	            }
	        }
	        public static void WriteCsvWithHeader(IEnumerable<global::test.csv.TestObject> items, string path, string[] header, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var h = BuildHeader(header, string.Empty);
	            using var sw = new StreamWriter(path, false, new UTF8Encoding(false));
	            sw.WriteLine(Polygen.Common.CsvUtils.Join(header, sep));
	            foreach (var it in items)
	            {
	                var row = ToRowWithHeader(h, it, gap);
	                sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep));
	            }
	        }
	        public static Polygen.Common.CsvIndexHeader BuildHeader(string[] header, string prefix)
	        {
	            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);
	            var root = new Polygen.Common.CsvIndexHeader();
	            root.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "id", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "name", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "active", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "score", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                var ch = new Polygen.Common.CsvIndexHeader();
	                ch.Index = -1;
	                ch.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	                for (int i=0;;i++)
	                {
	                    int __ix;
	                    if (!map.TryGetValue(prefix + "tags[" + i + "]", out __ix))
	                    {
	                        break;
	                    }
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    ch.IndexList.Add(leaf);
	                }
	                root.IndexList.Add(ch);
	            }
	            {
	                int __ix;
	                if (map.TryGetValue(prefix + "color", out __ix))
	                {
	                    var leaf = new Polygen.Common.CsvIndexHeader();
	                    leaf.Index = __ix;
	                    root.IndexList.Add(leaf);
	                }
	            }
	            {
	                var sub = global::Csv.test.csv.Point.BuildHeader(header, prefix + "location.");
	                if (sub.HasAny()) root.IndexList.Add(sub);
	            }
	            {
	                var ch = new Polygen.Common.CsvIndexHeader();
	                ch.Index = -1;
	                ch.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();
	                for (int i=0;;i++)
	                {
	                    var sub = global::Csv.test.csv.Point.BuildHeader(header, prefix + "history["+i+"].");
	                    if (!sub.HasAny()) break;
	                    ch.IndexList.Add(sub);
	                }
	                root.IndexList.Add(ch);
	            }
	            return root;
	        }

	        public static global::test.csv.TestObject FromRowWithHeader(Polygen.Common.CsvIndexHeader h, string[] row, Polygen.Common.CsvUtils.GapMode gap)
	        {
	            var obj = new global::test.csv.TestObject();
	            int __i = 0;
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.id = DataSourceFactory.ConvertValue<uint>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.name = DataSourceFactory.ConvertValue<string>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.active = DataSourceFactory.ConvertValue<bool>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.score = DataSourceFactory.ConvertValue<float>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	                {
	                    var list = new System.Collections.Generic.List<string>();
	                    if (__h != null && __h.IndexList != null)
	                    {
	                        for (int i = 0; i < __h.IndexList.Count; i++)
	                        {
	                            var subH = __h.IndexList[i];
	                            if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row))
	                            {
	                                if (i == 0 || gap == Polygen.Common.CsvUtils.GapMode.Break)
	                                {
	                                    break;
	                                }
	                                else
	                                {
	                                    continue;
	                                }
	                            }
	                            var v = DataSourceFactory.ConvertValue<string>(row[subH.Index]);
	                            list.Add(v);
	                        }
	                    }
	                    obj.tags = list;
	                }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (__h != null && __h.Index >= 0 && __h.Index < row.Length)
	            {
	                var __cell = row[__h.Index];
	                obj.color = DataSourceFactory.ConvertValue<global::test.csv.Color>(__cell);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	            if (!Polygen.Common.CsvUtils.HeaderHasValues(__h, row))
	            {
	                obj.location = null;
	            }
	            else
	            {
	                obj.location = global::Csv.test.csv.Point.FromRowWithHeader(__h, row, gap);
	            }
	            }
	            {
	                var __h = (h != null && h.IndexList != null && __i < h.IndexList.Count) ? h.IndexList[__i++] : null;
	                {
	                    var list = new System.Collections.Generic.List<global::test.csv.Point>();
	                    if (__h != null && __h.IndexList != null)
	                    {
	                        for (int i = 0; i < __h.IndexList.Count; i++)
	                        {
	                            var subH = __h.IndexList[i];
	                            if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row))
	                            {
	                                if (i == 0 || gap == Polygen.Common.CsvUtils.GapMode.Break)
	                                {
	                                    break;
	                                }
	                                else
	                                {
	                                    continue;
	                                }
	                            }
	                            var sub = global::Csv.test.csv.Point.FromRowWithHeader(subH, row, gap);
	                            list.Add(sub);
	                        }
	                    }
	                    obj.history = list;
	                }
	            }
	            return obj;
	        }
	        public static System.Collections.Generic.IEnumerable<global::test.csv.TestObject> ReadRowsWithHeader(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	        {
	            var lines = File.ReadAllLines(path);
	            if (lines.Length == 0)
	            {
	                yield break;
	            }
	            var header = lines[0].Split(sep);
	            var h = BuildHeader(header, string.Empty);
	            for (int i = 1; i < lines.Length; i++)
	            {
	                var row = lines[i].Split(sep);
	                if (!Polygen.Common.CsvUtils.HeaderHasValues(h, row))
	                {
	                    if (gap == Polygen.Common.CsvUtils.GapMode.Break)
	                    {
	                        break;
	                    }
	                    else
	                    {
	                        continue;
	                    }
	                }
	                yield return FromRowWithHeader(h, row, gap);
	            }
	        }
	        public static System.Collections.Generic.IEnumerable<global::test.csv.TestObject> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)
	            => ReadRowsWithHeader(path, sep, gap);

	}



}




