# 생성된 C# 코드 관리 스크립트

## 사용법

### 코드 검토용 복사
테스트나 검토를 위해 생성된 C# 코드를 `temp_output_review`로 복사합니다:

```powershell
.\scripts\copy_output_for_review.ps1
```

### 임시 폴더 정리
커밋 전에 `temp_output_review` 폴더를 삭제합니다:

```powershell
.\scripts\clean_temp_review.ps1
```

## 참고사항

- `output/csharp`: 실제 생성된 코드 (gitignore됨)
- `temp_output_review`: 임시 검토용 복사본 (gitignore됨)
- `tests/game_schema_compile_test`: `output/csharp`를 직접 참조하여 컴파일 테스트

## 워크플로우

1. 코드 생성:
   ```powershell
   cargo run -- --schema-path examples/game_schema.poly --lang csharp
   ```

2. (선택) 검토용 복사:
   ```powershell
   .\scripts\copy_output_for_review.ps1
   ```

3. 컴파일 테스트:
   ```powershell
   cd tests/game_schema_compile_test
   dotnet build
   ```

4. 커밋 전 정리:
   ```powershell
   .\scripts\clean_temp_review.ps1
   git add .
   git commit
   ```
