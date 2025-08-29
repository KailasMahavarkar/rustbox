#include <iostream>
using namespace std;

void printStarPattern(int N) {
    for (int i = 1; i <= N; ++i) {
        for (int j = 0; j < 2 * i - 1; ++j) cout << "*";
        cout << " ";
    }
    cout << endl;
}

int main() {
    int N;
    cin >> N;
    printStarPattern(N);
    return 0;
}