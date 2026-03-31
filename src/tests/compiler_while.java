public class compiler_while {
    public static void main(String[] args) {
        System.out.println("Start");

        int count = 5;
        while (count > 0) {
            System.out.println(count);
            count = count - 1;
        }
        
        System.out.println("Done");
    }
}