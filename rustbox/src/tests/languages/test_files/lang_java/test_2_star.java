import java.util.Scanner;

public class test_2_star {
    public static void printStarPattern(int N) {
        for (int i = 1; i <= N; i++) {
            if (i > 1) System.out.print(" ");
            for (int j = 0; j < 2 * i - 1; j++) {
                System.out.print("*");
            }
        }
        System.out.println();
    }
    
    public static void main(String[] args) {
        Scanner scanner = new Scanner(System.in);
        int N = scanner.nextInt();
        printStarPattern(N);
        scanner.close();
    }
}