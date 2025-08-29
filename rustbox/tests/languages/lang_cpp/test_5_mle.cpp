#include <iostream>
#include <vector>
using namespace std;

int main() {
    size_t size = 1;
    while (1) {
        try {
            vector<int> arr(size);
            size *= 2;
        } catch (bad_alloc& e) {
            cout << "Memory allocation failed at size = " << size << endl;
            break;
        }
    }
    return 0;
}