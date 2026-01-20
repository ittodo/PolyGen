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
	        ById.Add(row.Id, row);
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
	    public PointTable Points { get; } = new();
	    public TestObjectTable TestObjects { get; } = new();

	    public testcsvDataContainer()
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

}


