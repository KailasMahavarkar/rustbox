#include <iostream>
#include <string>
using namespace std;

void print_star_pattern(int N) {
    for (int i = 1; i <= N; i++) {
        if (i > 1) cout << " ";
        for (int j = 0; j < 2 * i - 1; j++) {
            cout << "*";
        }
    }
    cout << endl;
}

int main() {
    int N;
    cin >> N;
    print_star_pattern(N);
    return 0;
}