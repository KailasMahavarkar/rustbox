import java.util.ArrayList;

public class test_5_mle {
    public static void main(String[] args) {
        int size = 1;
        try {
            while (true) {
                ArrayList<Integer> arr = new ArrayList<>(size);
                for (int i = 0; i < size; i++) {
                    arr.add(i);
                }
                size *= 2;
                if (size <= 0) break;
            }
        } catch (OutOfMemoryError e) {
            System.out.println("OutOfMemoryError at size = " + size);
        }
    }
}