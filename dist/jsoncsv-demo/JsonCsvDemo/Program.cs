using System;
using System.Collections.Generic;
using Polygen.Common;

// Demo 3: Deeply nested arrays + dictionaries inside nested lists
var json = new List<object?>
{
    new Dictionary<string, object?>
    {
        ["user"] = new Dictionary<string, object?>
        {
            ["id"] = 100,
            ["name"] = "Kim",
            ["addresses"] = new List<object?>
            {
                new Dictionary<string, object?>
                {
                    ["city"] = "Seoul",
                    ["geo"] = new Dictionary<string, object?> { ["lat"] = 37.5, ["lng"] = 127.0 },
                    ["phones"] = new List<object?>
                    {
                        new Dictionary<string, object?> { ["type"] = "home", ["number"] = "010-1111-2222" },
                        new Dictionary<string, object?> { ["type"] = "work", ["number"] = "02-123-4567" }
                    }
                },
                new Dictionary<string, object?>
                {
                    ["city"] = "Busan",
                    ["geo"] = new Dictionary<string, object?> { ["lat"] = 35.1, ["lng"] = 129.0 }
                }
            }
        },
        ["orders"] = new List<object?>
        {
            new Dictionary<string, object?>
            {
                ["id"] = "A1",
                ["items"] = new List<object?>
                {
                    new Dictionary<string, object?> { ["sku"] = "S1", ["qty"] = 1 },
                    new Dictionary<string, object?> { ["sku"] = "S2", ["qty"] = 3 }
                },
                ["meta"] = new Dictionary<string, object?> { ["gift"] = true }
            }
        }
    },
    new Dictionary<string, object?>
    {
        ["user"] = new Dictionary<string, object?>
        {
            ["id"] = 101,
            ["name"] = "Lee",
            ["addresses"] = new List<object?>
            {
                new Dictionary<string, object?>
                {
                    ["city"] = "Incheon",
                    ["geo"] = new Dictionary<string, object?> { ["lat"] = 37.4, ["lng"] = 126.7 }
                }
            }
        },
        ["orders"] = new List<object?>
        {
            new Dictionary<string, object?>
            {
                ["id"] = "B9",
                ["items"] = new List<object?>
                {
                    new Dictionary<string, object?> { ["sku"] = "S3", ["qty"] = 2 }
                }
            }
        }
    }
};

var cfg = new JsonCsvConverter.Config
{
    ListStrategy = "dynamic", // 관측된 최대 인덱스까지 확장 (첫 번째 리스트 인덱스만)
    IncludeHeader = true
};

string csv = JsonCsvConverter.JsonToCsv(json, cfg);
Console.WriteLine(csv);
