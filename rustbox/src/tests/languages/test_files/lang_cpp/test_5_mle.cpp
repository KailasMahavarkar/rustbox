#include <iostream>
#include <vector>
using namespace std;

int main() {
    vector<vector<int>> data;
    int size = 1;
    
    try {
        while (true) {
            vector<int> arr(size, 0);
            data.push_back(arr);
            size *= 2;
            if (size <= 0) break;
        }
    } catch (const bad_alloc& e) {
        cout << "Memory allocation failed" << endl;
    } catch (const exception& e) {
        cout << "Error: " << e.what() << endl;
    }
    
    return 0;
}