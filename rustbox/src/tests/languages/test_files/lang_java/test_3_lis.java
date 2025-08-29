import java.util.Arrays;

public class test_3_lis {
    public static int longestIncreasingSubsequence(int[] arr) {
        if (arr.length == 0) return 0;
        
        int n = arr.length;
        int[] dp = new int[n];
        Arrays.fill(dp, 1);
        
        for (int i = 1; i < n; i++) {
            for (int j = 0; j < i; j++) {
                if (arr[j] < arr[i]) {
                    dp[i] = Math.max(dp[i], dp[j] + 1);
                }
            }
        }
        
        return Arrays.stream(dp).max().orElse(0);
    }
    
    public static void main(String[] args) {
        int[] arr = {10, 9, 2, 5, 3, 7, 101, 18};
        int result = longestIncreasingSubsequence(arr);
        System.out.println("LIS length: " + result);
    }
}