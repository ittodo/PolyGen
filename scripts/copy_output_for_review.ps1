# temp_output_review 폴더로 생성된 C# 코드 복사
Remove-Item -Path "temp_output_review" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item -Path "output\csharp" -Destination "temp_output_review" -Recurse -Force
Write-Host "✅ C# 코드를 temp_output_review로 복사했습니다." -ForegroundColor Green
