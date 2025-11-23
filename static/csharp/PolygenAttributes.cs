using System;

namespace Polygen.Common
{
    /// <summary>
    /// 테이블이나 타입에 태그를 붙일 수 있음을 나타내는 어트리뷰트입니다.
    /// PolyGen의 @taggable 어노테이션에 해당합니다.
    /// </summary>
    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct, AllowMultiple = false)]
    public class taggableAttribute : Attribute
    {
    }

    /// <summary>
    /// 데이터 로드 소스를 지정하는 어트리뷰트입니다.
    /// PolyGen의 @load 어노테이션에 해당합니다.
    /// </summary>
    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct, AllowMultiple = false)]
    public class loadAttribute : Attribute
    {
        /// <summary>
        /// 로드 타입 (예: "DB", "File" 등)
        /// </summary>
        public string type { get; set; } = "";

        /// <summary>
        /// 로드 경로 (예: 테이블 이름, 파일 경로 등)
        /// </summary>
        public string path { get; set; } = "";

        public loadAttribute()
        {
        }
    }

    /// <summary>
    /// 행(row)들을 연결 리스트 형태로 연결하는 방법을 지정하는 어트리뷰트입니다.
    /// PolyGen의 @link_rows 어노테이션에 해당합니다.
    /// 주로 N:M 연결 테이블에서 사용됩니다.
    /// </summary>
    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct, AllowMultiple = false)]
    public class link_rowsAttribute : Attribute
    {
        /// <summary>
        /// 파티션 기준이 되는 필드 이름
        /// </summary>
        public string partition_by { get; set; } = "";

        /// <summary>
        /// 링크에 사용할 필드 이름
        /// </summary>
        public string link_with { get; set; } = "";

        public link_rowsAttribute()
        {
        }
    }

    /// <summary>
    /// 인덱스를 지정하는 어트리뷰트입니다.
    /// System.ComponentModel.DataAnnotations.Schema.IndexAttribute를 확장합니다.
    /// </summary>
    [AttributeUsage(AttributeTargets.Property | AttributeTargets.Field, AllowMultiple = false)]
    public class IndexAttribute : Attribute
    {
        /// <summary>
        /// 인덱스가 고유(unique)한지 여부
        /// </summary>
        public bool IsUnique { get; set; } = false;

        /// <summary>
        /// 인덱스 이름 (선택 사항)
        /// </summary>
        public string? Name { get; set; }

        public IndexAttribute()
        {
        }

        public IndexAttribute(string name)
        {
            Name = name;
        }
    }
}
