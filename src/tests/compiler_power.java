public class compiler_power {
    public static int power(int base, int exp) {
        int result = 1;
        while (exp > 0) {
            result = result * base;
            exp = exp - 1;
        }
        return result;
    }

    public static void main(String[] args) {
        int val = power(2, 5);

        System.out.println(val);
    }
}