#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

int longest_increasing_subsequence(vector<int>& arr) {
    if (arr.empty()) return 0;
    
    int n = arr.size();
    vector<int> dp(n, 1);
    
    for (int i = 1; i < n; i++) {
        for (int j = 0; j < i; j++) {
            if (arr[j] < arr[i]) {
                dp[i] = max(dp[i], dp[j] + 1);
            }
        }
    }
    
    return *max_element(dp.begin(), dp.end());
}

int main() {
    vector<int> arr = {10, 9, 2, 5, 3, 7, 101, 18};
    int result = longest_increasing_subsequence(arr);
    cout << "LIS length: " << result << endl;
    return 0;
}