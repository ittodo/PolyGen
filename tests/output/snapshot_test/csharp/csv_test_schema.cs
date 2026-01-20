using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using Polygen.Common;
namespace test.csv
{
	// This template generates a C# enum definition.
	// It expects 'e' to be an EnumDef object.


	public enum Color
	{
	    RED,
	    GREEN,
	    BLUE,

	}


	public class Point : IDataRow
	{
	    private IDataContainer? _container;

	    public void SetContainer(IDataContainer container)
	    {
	        _container = container;
	    }

	    public int x;
	    public int y;


	}




	public class TestObject : IDataRow
	{
	    private IDataContainer? _container;

	    public void SetContainer(IDataContainer container)
	    {
	        _container = container;
	    }

	    [Key]
	    public uint id;
	    public string name;
	    public bool active;
	    public float score;
	    public List<string> tags;
	    public Color color;
	    public Point location;
	    public List<Point> history;


	}


}



