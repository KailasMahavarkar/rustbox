def print_star_pattern(N):
    result = []
    for i in range(1, N + 1):
        result.append('*' * (2 * i - 1))
    print(' '.join(result))

N = int(input())
print_star_pattern(N)