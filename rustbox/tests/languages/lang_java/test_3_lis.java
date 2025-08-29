public class test_3_lis {
    public static int lis(int[] arr) {
        int n = arr.length;
        int[] dp = new int[n];
        for (int i = 0; i < n; i++) dp[i] = 1;
        
        for (int i = 1; i < n; i++) {
            for (int j = 0; j < i; j++) {
                if (arr[i] > arr[j] && dp[i] < dp[j] + 1) {
                    dp[i] = dp[j] + 1;
                }
            }
        }
        
        int max = 1;
        for (int i = 0; i < n; i++) {
            if (dp[i] > max) max = dp[i];
        }
        return max;
    }
    
    public static void main(String[] args) {
        int[] arr = {10, 22, 9, 33, 21, 50, 41, 60};
        System.out.println(lis(arr));
    }
}