using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using test.csv;

public class Program
{
    public static void Main(string[] args)
    {
        try 
        {
            string inputPath = Path.GetFullPath("../../tests/test_data/sample_input.json");
            string expectedPath = Path.GetFullPath("../../tests/test_data/expected_output.csv");
            string actualPath = "actual_output.csv";

            if (!File.Exists(inputPath)) 
            {
                Console.WriteLine($"Error: Input file not found at {inputPath}");
                Environment.Exit(1);
            }

            Console.WriteLine($"Reading input from: {inputPath}");
            string json = File.ReadAllText(inputPath);
            
            var options = new JsonSerializerOptions { PropertyNameCaseInsensitive = true, IncludeFields = true };
            options.Converters.Add(new System.Text.Json.Serialization.JsonStringEnumConverter());
            var data = JsonSerializer.Deserialize<List<TestObject>>(json, options);

            if (data == null) 
            {
                Console.WriteLine("Error: Failed to deserialize JSON (null result)");
                Environment.Exit(1);
            }

            Console.WriteLine($"Deserialized {data.Count} items.");

            Console.WriteLine("Writing CSV...");
            Csv.test.csv.TestObject.WriteCsv(data, actualPath);

            Console.WriteLine("Verifying output...");
            string expected = File.ReadAllText(expectedPath).Replace("\r\n", "\n").Trim();
            string actual = File.ReadAllText(actualPath).Replace("\r\n", "\n").Trim();

            if (expected == actual)
            {
                Console.WriteLine("SUCCESS: Output matches expected CSV.");
            }
            else
            {
                Console.WriteLine("FAILURE: Output does not match expected CSV.");
                Console.WriteLine("--- Expected ---");
                Console.WriteLine(expected);
                Console.WriteLine("--- Actual ---");
                Console.WriteLine(actual);
                Environment.Exit(1);
            }
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Exception: {ex}");
            Environment.Exit(1);
        }
    }
}
