import java.util.ArrayList;
import java.util.List;

public class test_5_mle {
    public static void main(String[] args) {
        List<List<Integer>> data = new ArrayList<>();
        int size = 1;
        
        try {
            while (true) {
                List<Integer> arr = new ArrayList<>();
                for (int i = 0; i < size; i++) {
                    arr.add(0);
                }
                data.add(arr);
                size *= 2;
                if (size <= 0) break;
            }
        } catch (OutOfMemoryError e) {
            System.out.println("Memory allocation failed");
        } catch (Exception e) {
            System.out.println("Error: " + e.getMessage());
        }
    }
}