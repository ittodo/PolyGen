using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.IO;
using System.Text;
using Polygen.Common;
namespace test.csv
{
    public static class BinaryReaders
    {
        public static global::test.csv.Point ReadPoint(this BinaryReader br)
        {
            var obj = new global::test.csv.Point();
                obj.x = br.ReadInt32();
                obj.y = br.ReadInt32();

            return obj;
        }

        public static global::test.csv.TestObject ReadTestObject(this BinaryReader br)
        {
            var obj = new global::test.csv.TestObject();
                obj.id = br.ReadUInt32();
                obj.name = BinaryUtils.ReadUtf8String(br);
                obj.active = br.ReadBoolean();
                obj.score = br.ReadSingle();
                {
                    obj.tags = BinaryUtils.ReadList<string>(br, BinaryUtils.ReadUtf8String);
                }
                obj.color = BinaryUtils.ReadEnumInt32<test.csv.Color>(br);
                obj.location = test.csv.BinaryReaders.ReadPoint(br);
                {
                    obj.history = BinaryUtils.ReadList<global::test.csv.Point>(br, test.csv.BinaryReaders.ReadPoint);
                }

            return obj;
        }


    }
}
