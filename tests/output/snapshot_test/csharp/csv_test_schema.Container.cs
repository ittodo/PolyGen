using System;
using System.Collections.Generic;
using System.Linq;
using Polygen.Common;

namespace test.csv.Container
{
	/// <summary>
	/// Data table for Point entities.
	/// </summary>
	public class PointTable : DataTableBase<Point>
	{
	    /// <summary>
	    /// Adds a row to the table.
	    /// </summary>
	    public void Add(Point row)
	    {
	        AddRowInternal(row);
	    }

	    protected override void OnRowAdded(Point row)
	    {
	    }

	    protected override void OnCleared()
	    {
	    }

	}

	/// <summary>
	/// Data table for TestObject entities.
	/// </summary>
	public class TestObjectTable : DataTableBase<TestObject>
	{
	    public UniqueIndex<uint, TestObject> ById { get; } = new();

	    /// <summary>
	    /// Adds a row to the table.
	    /// </summary>
	    public void Add(TestObject row)
	    {
	        AddRowInternal(row);
	    }

	    protected override void OnRowAdded(TestObject row)
	    {
	        ById.Add(row.id, row);
	    }

	    protected override void OnCleared()
	    {
	        ById.Clear();
	    }

	}

	/// <summary>
	/// Root data container for test.csv entities.
	/// </summary>
	public class testcsvDataContainer : IDataContainer
	{
	    /// <summary>
	    /// Gets or sets the root directory for resolving relative file paths.
	    /// </summary>
	    public string RootDirectory { get; set; } = ".";

	    public PointTable Points { get; } = new();
	    public TestObjectTable TestObjects { get; } = new();

	    public testcsvDataContainer(string rootDirectory = ".")
	    {
	        RootDirectory = rootDirectory;
	        Points.SetContainer(this);
	        TestObjects.SetContainer(this);
	    }

	    /// <summary>
	    /// Clears all data from all tables.
	    /// </summary>
	    public void Clear()
	    {
	        Points.Clear();
	        TestObjects.Clear();
	    }

	}

}


namespace CsvTestSchema.Container
{
	/// <summary>
	/// Interface for containers that have a Point table.
	/// </summary>
	public interface IHasPointTable
	{
	    global::test.csv.Container.PointTable Points { get; }
	}

	/// <summary>
	/// Interface for containers that have a TestObject table.
	/// </summary>
	public interface IHasTestObjectTable
	{
	    global::test.csv.Container.TestObjectTable TestObjects { get; }
	}


	/// <summary>
	/// Root data container for all entities in this schema file.
	/// Provides unified access to all tables with their indexes.
	/// </summary>
	public class CsvTestSchemaDataContainer : IDataContainer, IHasPointTable, IHasTestObjectTable
	{
	    public global::test.csv.Container.PointTable Points { get; } = new();
	    public global::test.csv.Container.TestObjectTable TestObjects { get; } = new();

	    public CsvTestSchemaDataContainer()
	    {
	        Points.SetContainer(this);
	        TestObjects.SetContainer(this);
	    }

	    /// <summary>
	    /// Clears all data from all tables.
	    /// </summary>
	    public void Clear()
	    {
	        Points.Clear();
	        TestObjects.Clear();
	    }

	}


	/// <summary>
	/// Data loader for populating the CsvTestSchemaDataContainer from CSV/JSON files.
	/// Supports pattern-based loading with wildcards (e.g., "data/*.csv", "data/players/").
	/// </summary>
	public static class CsvTestSchemaDataLoader
	{
	    /// <summary>
	    /// Loads Point data from a CSV file.
	    /// </summary>
	    public static void LoadPointsFromCsv(CsvTestSchemaDataContainer container, string filePath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	        foreach (var item in Csv.test.csv.Point.ReadRowsWithHeader(filePath, separator, gapMode))
	        {
	            container.Points.Add(item);
	        }
	    }

	    /// <summary>
	    /// Loads Point data from CSV files matching a pattern.
	    /// Supports: single file, wildcards (*.csv), directories (data/).
	    /// Files are sorted alphabetically and merged sequentially.
	    /// </summary>
	    /// <exception cref="DuplicateKeyException">Thrown when duplicate primary keys are found across files.</exception>
	    public static void LoadPointsFromCsvPattern(CsvTestSchemaDataContainer container, string pattern, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, ".csv");
	        foreach (var file in files)
	        {
	            foreach (var item in Csv.test.csv.Point.ReadRowsWithHeader(file, separator, gapMode))
	            {
	                container.Points.Add(item);
	            }
	        }
	    }

	    /// <summary>
	    /// Loads Point data from a JSON file.
	    /// </summary>
	    public static void LoadPointsFromJson(CsvTestSchemaDataContainer container, string filePath)
	    {
	        var json = File.ReadAllText(filePath);
	        var items = System.Text.Json.JsonSerializer.Deserialize<List<global::test.csv.Point>>(json);
	        if (items != null)
	        {
	            foreach (var item in items)
	            {
	                container.Points.Add(item);
	            }
	        }
	    }

	    /// <summary>
	    /// Loads Point data from JSON files matching a pattern.
	    /// </summary>
	    /// <exception cref="DuplicateKeyException">Thrown when duplicate primary keys are found across files.</exception>
	    public static void LoadPointsFromJsonPattern(CsvTestSchemaDataContainer container, string pattern)
	    {
	        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, ".json");
	        foreach (var file in files)
	        {
	            var json = File.ReadAllText(file);
	            var items = System.Text.Json.JsonSerializer.Deserialize<List<global::test.csv.Point>>(json);
	            if (items != null)
	            {
	                foreach (var item in items)
	                {
	                    container.Points.Add(item);
	                }
	            }
	        }
	    }

	    /// <summary>
	    /// Loads TestObject data from a CSV file.
	    /// </summary>
	    public static void LoadTestObjectsFromCsv(CsvTestSchemaDataContainer container, string filePath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	        foreach (var item in Csv.test.csv.TestObject.ReadRowsWithHeader(filePath, separator, gapMode))
	        {
	            container.TestObjects.Add(item);
	        }
	    }

	    /// <summary>
	    /// Loads TestObject data from CSV files matching a pattern.
	    /// Supports: single file, wildcards (*.csv), directories (data/).
	    /// Files are sorted alphabetically and merged sequentially.
	    /// </summary>
	    /// <exception cref="DuplicateKeyException">Thrown when duplicate primary keys are found across files.</exception>
	    public static void LoadTestObjectsFromCsvPattern(CsvTestSchemaDataContainer container, string pattern, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, ".csv");
	        var seenKeys = new HashSet<object>();
	        string? firstFile = null;
	        foreach (var file in files)
	        {
	            foreach (var item in Csv.test.csv.TestObject.ReadRowsWithHeader(file, separator, gapMode))
	            {
	                var key = item.id;
	                if (seenKeys.Contains(key))
	                {
	                    throw new DuplicateKeyException(key?.ToString() ?? "(null)", firstFile ?? file, file);
	                }
	                seenKeys.Add(key);
	                if (firstFile == null) firstFile = file;
	                container.TestObjects.Add(item);
	            }
	        }
	    }

	    /// <summary>
	    /// Loads TestObject data from a JSON file.
	    /// </summary>
	    public static void LoadTestObjectsFromJson(CsvTestSchemaDataContainer container, string filePath)
	    {
	        var json = File.ReadAllText(filePath);
	        var items = System.Text.Json.JsonSerializer.Deserialize<List<global::test.csv.TestObject>>(json);
	        if (items != null)
	        {
	            foreach (var item in items)
	            {
	                container.TestObjects.Add(item);
	            }
	        }
	    }

	    /// <summary>
	    /// Loads TestObject data from JSON files matching a pattern.
	    /// </summary>
	    /// <exception cref="DuplicateKeyException">Thrown when duplicate primary keys are found across files.</exception>
	    public static void LoadTestObjectsFromJsonPattern(CsvTestSchemaDataContainer container, string pattern)
	    {
	        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, ".json");
	        var seenKeys = new HashSet<object>();
	        string? firstFile = null;
	        foreach (var file in files)
	        {
	            var json = File.ReadAllText(file);
	            var items = System.Text.Json.JsonSerializer.Deserialize<List<global::test.csv.TestObject>>(json);
	            if (items != null)
	            {
	                foreach (var item in items)
	                {
	                    var key = item.id;
	                    if (seenKeys.Contains(key))
	                    {
	                        throw new DuplicateKeyException(key?.ToString() ?? "(null)", firstFile ?? file, file);
	                    }
	                    seenKeys.Add(key);
	                    if (firstFile == null) firstFile = file;
	                    container.TestObjects.Add(item);
	                }
	            }
	        }
	    }

	    /// <summary>
	    /// Loads all tables using patterns from @load annotations.
	    /// Files are resolved relative to container.RootDirectory.
	    /// </summary>
	    public static void LoadAll(CsvTestSchemaDataContainer container, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	    }

	    /// <summary>
	    /// Loads all tables from CSV files in a directory.
	    /// Files should be named: {TableName}.csv
	    /// </summary>
	    public static void LoadAllFromCsvDirectory(CsvTestSchemaDataContainer container, string directoryPath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)
	    {
	        var pointPath = Path.Combine(directoryPath, "Point.csv");
	        if (File.Exists(pointPath))
	            LoadPointsFromCsv(container, pointPath, separator, gapMode);

	        var testobjectPath = Path.Combine(directoryPath, "TestObject.csv");
	        if (File.Exists(testobjectPath))
	            LoadTestObjectsFromCsv(container, testobjectPath, separator, gapMode);

	    }

	    /// <summary>
	    /// Loads all tables from JSON files in a directory.
	    /// Files should be named: {TableName}.json
	    /// </summary>
	    public static void LoadAllFromJsonDirectory(CsvTestSchemaDataContainer container, string directoryPath)
	    {
	        var pointPath = Path.Combine(directoryPath, "Point.json");
	        if (File.Exists(pointPath))
	            LoadPointsFromJson(container, pointPath);

	        var testobjectPath = Path.Combine(directoryPath, "TestObject.json");
	        if (File.Exists(testobjectPath))
	            LoadTestObjectsFromJson(container, testobjectPath);

	    }

	}

}
