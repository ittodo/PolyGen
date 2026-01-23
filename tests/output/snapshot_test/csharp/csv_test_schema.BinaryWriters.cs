using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.IO;
using System.Text;
using Polygen.Common;
namespace test.csv
{
    public static class BinaryWriters
    {
		public static void WritePoint(this BinaryWriter bw, global::test.csv.Point obj)
		{
		        bw.Write(obj.x);
		        bw.Write(obj.y);


		}



		public static void WriteTestObject(this BinaryWriter bw, global::test.csv.TestObject obj)
		{
		        bw.Write(obj.id);
		        BinaryUtils.WriteUtf8String(bw, obj.name);
		        bw.Write(obj.active);
		        bw.Write(obj.score);
		        BinaryUtils.WriteList<string>(bw, obj.tags, BinaryUtils.WriteUtf8String);
		        BinaryUtils.WriteEnumInt32<test.csv.Color>(bw, obj.color);
		        test.csv.BinaryWriters.WritePoint(bw, obj.location);
		        BinaryUtils.WriteList<global::test.csv.Point>(bw, obj.history, test.csv.BinaryWriters.WritePoint);


		}




    }
}




