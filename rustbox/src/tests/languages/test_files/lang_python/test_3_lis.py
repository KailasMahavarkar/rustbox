def longest_increasing_subsequence(arr):
    if not arr:
        return 0
    
    n = len(arr)
    dp = [1] * n
    
    for i in range(1, n):
        for j in range(i):
            if arr[j] < arr[i]:
                dp[i] = max(dp[i], dp[j] + 1)
    
    return max(dp)

# Test with sample input
arr = [10, 9, 2, 5, 3, 7, 101, 18]
result = longest_increasing_subsequence(arr)
print(f"LIS length: {result}")