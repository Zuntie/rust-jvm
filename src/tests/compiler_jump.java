public class compiler_jump {
    public static void main(String[] args) {
        System.out.println("Start");

        int a = 5;
        int b = 10;

        int sum = a + b;
        System.out.println(sum);

        int diff = a - b;
        System.out.println(diff);

        if (b < a) {
            System.out.println("Jump");
        } else {
            System.out.println("No Jump");
        }
        
        System.out.println("Done");
    }
}