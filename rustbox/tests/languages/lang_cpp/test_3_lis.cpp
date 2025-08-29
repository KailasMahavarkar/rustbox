#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

int lis(vector<int>& arr) {
    int n = arr.size();
    vector<int> dp(n, 1);
    
    for (int i = 1; i < n; i++) {
        for (int j = 0; j < i; j++) {
            if (arr[i] > arr[j] && dp[i] < dp[j] + 1) {
                dp[i] = dp[j] + 1;
            }
        }
    }
    
    return *max_element(dp.begin(), dp.end());
}

int main() {
    vector<int> arr = {10, 22, 9, 33, 21, 50, 41, 60};
    cout << lis(arr) << endl;
    return 0;
}