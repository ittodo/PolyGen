using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;
using Polygen.Common;

namespace PolyGen.Examples
{
    /// <summary>
    /// Demonstrates how to convert JSON data to CSV using PolyGen-generated mappers.
    /// Usage: JsonToCsvDemo <input.json> <output.csv> <schema_namespace.class_name>
    /// Example: JsonToCsvDemo data.json output.csv Game.Player
    /// </summary>
    public class JsonToCsvDemo
    {
        public static void Run(string inputPath, string outputPath, string typeName)
        {
            if (!File.Exists(inputPath))
            {
                Console.WriteLine($"Error: Input file '{inputPath}' not found.");
                return;
            }

            try
            {
                string json = File.ReadAllText(inputPath);
                var options = new JsonSerializerOptions 
                { 
                    PropertyNameCaseInsensitive = true,
                    IncludeFields = true 
                };
                options.Converters.Add(new JsonStringEnumConverter());

                // Note: In a real dynamic scenario, you might use reflection to find the correct type and mapper.
                // For this demo, we assume a specific type or use a generic approach if possible.
                // Since generated mappers are static, we can't easily use generics without reflection or a common interface.
                // Here we show the pattern using the generated 'TestObject' from our test schema as an example.
                
                Console.WriteLine($"Reading JSON from {inputPath}...");

                // Example: Deserializing to a list of objects (using dynamic or a known type)
                // For demonstration, we'll use the generated 'test.csv.TestObject' if the typeName matches,
                // otherwise we'll show a generic error.
                
                if (typeName == "test.csv.TestObject")
                {
                    var data = JsonSerializer.Deserialize<List<test.csv.TestObject>>(json, options);
                    if (data != null)
                    {
                        Console.WriteLine($"Converting {data.Count} items to CSV...");
                        Csv.test.csv.TestObject.WriteCsv(data, outputPath);
                        Console.WriteLine($"Successfully wrote CSV to {outputPath}");
                    }
                    else
                    {
                        Console.WriteLine("Error: Deserialized data is null.");
                    }
                }
                else
                {
                    Console.WriteLine($"Error: Unknown or unsupported type '{typeName}'. This demo currently only supports 'test.csv.TestObject'.");
                    Console.WriteLine("To support more types, add them to the switch case in JsonToCsvDemo.cs or use reflection.");
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Exception: {ex.Message}");
                Console.WriteLine(ex.StackTrace);
            }
        }

        public static void Main(string[] args)
        {
            if (args.Length < 3)
            {
                Console.WriteLine("Usage: JsonToCsvDemo <input.json> <output.csv> <full_type_name>");
                return;
            }

            Run(args[0], args[1], args[2]);
        }
    }
}
